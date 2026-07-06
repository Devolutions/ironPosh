//! Deterministic latency bench for the serial (single-connection) session loop.
//!
//! Drives the real `start_serial_session_loop` (via the public
//! `open_task_serial` API) against an in-process fake WinRM server that honors
//! the `OperationTimeout` of each Receive, so the bench measures exactly the
//! scheduling latency the loop adds on top of the wire:
//!
//! - `output_latency_ms` — server-has-data → `PipelineOutput` event delivered
//! - `input_latency_ms`  — Invoke issued → Command request reaches the server
//! - `kill_signal_ms`    — Kill issued → Signal request reaches the server
//! - request counts      — polling efficiency (receives / timeouts)
//!
//! Run: `cargo run --release -p ironposh-async --example serial_latency_bench`
//! One process run = one bench run; repeat N times per the bench doc.

#![allow(clippy::significant_drop_tightening)] // bench: lock scopes are already minimal
#![allow(
    clippy::items_after_statements,
    clippy::large_futures,
    clippy::large_stack_frames
)]

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures::{FutureExt, StreamExt, join};
use futures_timer::Delay;
use ironposh_async::{HttpClient, RemoteAsyncPowershellClient};
use ironposh_client_core::connector::active_session::UserEvent;
use ironposh_client_core::connector::connection_pool::TrySend;
use ironposh_client_core::connector::http::HttpResponseTargeted;
use ironposh_psrp::ps_value::PsObjectWithType;
use ironposh_psrp::{
    ApplicationPrivateData, PipelineOutput, PipelineStateMessage, PsPrimitiveValue, PsValue,
    RunspacePoolStateMessage, RunspacePoolStateValue, SessionCapability,
};
use ironposh_test_support::fake_server;
use uuid::Uuid;

// ── Scenario scripting ──────────────────────────────────────────────────────

/// Timed output plan for one pipeline, offsets relative to Command arrival.
#[derive(Clone)]
struct PipelineScript {
    /// (offset, payload) — payload strings must be globally unique.
    outputs: Vec<(Duration, String)>,
    /// When the pipeline reports Completed. `None` = runs until killed.
    complete_at: Option<Duration>,
}

struct CommandRun {
    started_at: Instant,
    script: PipelineScript,
    delivered: usize,
    completed: bool,
}

#[derive(Default)]
struct Records {
    /// payload → instant the server had it ready
    ready_at: HashMap<String, Instant>,
    command_arrived_at: Vec<Instant>,
    signal_arrived_at: Vec<Instant>,
    receive_count: u64,
    timeout_count: u64,
    request_count: u64,
}

struct ServerState {
    rpid: Option<Uuid>,
    handshake_receive_done: bool,
    scripts: VecDeque<PipelineScript>,
    runs: HashMap<Uuid, CommandRun>,
    object_id: u64,
    records: Records,
}

struct FakeWinRmServer {
    state: Mutex<ServerState>,
}

impl FakeWinRmServer {
    fn new(scripts: Vec<PipelineScript>) -> Self {
        Self {
            state: Mutex::new(ServerState {
                rpid: None,
                handshake_receive_done: false,
                scripts: scripts.into(),
                runs: HashMap::new(),
                object_id: 100,
                records: Records::default(),
            }),
        }
    }
}

// ── Request parsing helpers (plaintext XML — Basic auth + HttpInsecure) ────

fn extract_between<'a>(haystack: &'a str, prefix: &str, suffix: char) -> Option<&'a str> {
    let start = haystack.find(prefix)? + prefix.len();
    let rest = &haystack[start..];
    let end = rest.find(suffix)?;
    Some(&rest[..end])
}

fn extract_action(body: &str) -> &'static str {
    for (needle, name) in [
        ("transfer/Create", "create"),
        ("shell/Command", "command"),
        ("shell/Receive", "receive"),
        ("shell/Signal", "signal"),
        ("transfer/Delete", "delete"),
    ] {
        if body.contains(needle) {
            return name;
        }
    }
    "other"
}

fn extract_operation_timeout(body: &str) -> Duration {
    extract_between(body, "PT", 'S')
        .and_then(|s| s.parse::<f64>().ok())
        .map_or(Duration::from_millis(250), Duration::from_secs_f64)
}

fn extract_command_id(body: &str) -> Option<Uuid> {
    extract_between(body, "CommandId=\"", '"').and_then(|s| s.parse().ok())
}

fn extract_message_id(body: &str) -> Option<String> {
    let inner = extract_between(body, "<a:MessageID>", '<')?;
    Some(inner.trim().to_owned())
}

// ── Fake server behavior ────────────────────────────────────────────────────

/// Newtype so `HttpClient` can be implemented for a shared server (orphan rule).
#[derive(Clone)]
struct SharedServer(std::sync::Arc<FakeWinRmServer>);

impl HttpClient for SharedServer {
    fn send_request(
        &self,
        try_send: TrySend,
    ) -> impl Future<Output = anyhow::Result<HttpResponseTargeted>> {
        let server = std::sync::Arc::clone(&self.0);
        async move {
            let (request, conn_id) = fake_server::expect_just_send(try_send);
            let body = request
                .body
                .as_ref()
                .and_then(|b| b.as_str().ok())
                .unwrap_or_default()
                .to_owned();

            {
                let mut st = server.state.lock().unwrap();
                st.records.request_count += 1;
            }

            let action = extract_action(&body);
            let xml = match action {
                "create" => {
                    let rpid = fake_server::extract_shell_id(&body);
                    server.state.lock().unwrap().rpid = Some(rpid);
                    include_str!("../../ironposh-client-core/tests/resources/resource_created.xml")
                        .to_owned()
                }
                "command" => {
                    let now = Instant::now();
                    let command_id =
                        extract_command_id(&body).expect("Command request must carry a CommandId");
                    let mut st = server.state.lock().unwrap();
                    let script = st
                        .scripts
                        .pop_front()
                        .expect("more Commands than scripted pipelines");
                    for (offset, payload) in &script.outputs {
                        st.records.ready_at.insert(payload.clone(), now + *offset);
                    }
                    st.records.command_arrived_at.push(now);
                    st.runs.insert(
                        command_id,
                        CommandRun {
                            started_at: now,
                            script,
                            delivered: 0,
                            completed: false,
                        },
                    );
                    fake_server::command_response_xml(command_id)
                }
                "signal" => {
                    let now = Instant::now();
                    let command_id = extract_command_id(&body);
                    let relates_to =
                        extract_message_id(&body).expect("Signal request must carry a MessageID");
                    let mut st = server.state.lock().unwrap();
                    st.records.signal_arrived_at.push(now);
                    // Killing a live pipeline makes it report Completed on the
                    // next Receive (mock simplification of Stopped).
                    if let Some(run) = command_id.and_then(|id| st.runs.get_mut(&id))
                        && run.script.complete_at.is_none()
                    {
                        run.script.complete_at = Some(now - run.started_at);
                    }
                    fake_server::signal_response_xml(&relates_to)
                }
                "receive" => server.handle_receive(&body).await,
                _ => fake_server::timeout_fault_xml(),
            };

            Ok(fake_server::xml_response(conn_id, xml))
        }
    }
}

impl FakeWinRmServer {
    /// Serve a Receive: respond the moment scripted data is due, otherwise
    /// hold the request until its own OperationTimeout and return a
    /// `w:TimedOut` fault — exactly like a real WSMan server.
    async fn handle_receive(&self, body: &str) -> String {
        let op_timeout = extract_operation_timeout(body);
        let deadline = Instant::now() + op_timeout;
        let command_id = extract_command_id(body);

        {
            let mut st = self.state.lock().unwrap();
            st.records.receive_count += 1;

            // Handshake receive: runspace-pool stream before the pool opened.
            if command_id.is_none() && !st.handshake_receive_done {
                st.handshake_receive_done = true;
                let rpid = st.rpid.expect("Create must precede Receive");
                let session_capability = SessionCapability {
                    protocol_version: "2.3".to_owned(),
                    ps_version: "2.0".to_owned(),
                    serialization_version: "1.1.0.1".to_owned(),
                    time_zone: None,
                };
                let private_data = ApplicationPrivateData::new();
                let opened = RunspacePoolStateMessage::builder()
                    .runspace_state(RunspacePoolStateValue::Opened)
                    .build();
                return fake_server::receive_response_xml(
                    rpid,
                    &[&session_capability, &private_data, &opened],
                );
            }
        }

        let Some(command_id) = command_id else {
            // Runspace-pool long-poll with nothing to say: hold, then time out.
            Delay::new(op_timeout).await;
            let mut st = self.state.lock().unwrap();
            st.records.timeout_count += 1;
            return fake_server::timeout_fault_xml();
        };

        loop {
            let now = Instant::now();
            enum Next {
                Respond(String),
                WaitUntil(Instant),
            }

            let next = {
                let mut st = self.state.lock().unwrap();
                let rpid = st.rpid.expect("Create must precede Receive");
                let object_id = st.object_id;
                let run = st.runs.get_mut(&command_id).expect("unknown CommandId");
                let elapsed = now - run.started_at;

                let due_outputs: Vec<String> = run.script.outputs[run.delivered..]
                    .iter()
                    .take_while(|(offset, _)| *offset <= elapsed)
                    .map(|(_, payload)| payload.clone())
                    .collect();
                let complete_due = !run.completed
                    && run
                        .script
                        .complete_at
                        .is_some_and(|offset| offset <= elapsed);

                if !due_outputs.is_empty() || complete_due {
                    run.delivered += due_outputs.len();
                    run.completed |= complete_due;

                    let outputs: Vec<PipelineOutput> = due_outputs
                        .iter()
                        .map(|payload| PipelineOutput {
                            data: PsValue::Primitive(PsPrimitiveValue::Str(payload.clone())),
                        })
                        .collect();
                    let state_msg = PipelineStateMessage::completed();
                    let mut messages: Vec<&dyn PsObjectWithType> =
                        outputs.iter().map(|o| o as &dyn PsObjectWithType).collect();
                    if complete_due {
                        messages.push(&state_msg);
                    }

                    let xml = fake_server::pipeline_receive_response_xml(
                        rpid,
                        command_id,
                        &messages,
                        complete_due,
                        object_id,
                    );
                    st.object_id += messages.len() as u64;
                    Next::Respond(xml)
                } else {
                    // Nothing due: wake at the earliest of next scripted event
                    // or this request's OperationTimeout.
                    let next_event = run.script.outputs[run.delivered..]
                        .first()
                        .map(|(offset, _)| run.started_at + *offset);
                    let next_complete = if run.completed {
                        None
                    } else {
                        run.script.complete_at.map(|offset| run.started_at + offset)
                    };
                    let wake_at = [next_event, next_complete]
                        .into_iter()
                        .flatten()
                        .min()
                        .map_or(deadline, |t| t.min(deadline));
                    Next::WaitUntil(wake_at)
                }
            };

            match next {
                Next::Respond(xml) => return xml,
                Next::WaitUntil(wake_at) => {
                    if wake_at >= deadline {
                        let sleep = deadline.saturating_duration_since(now);
                        Delay::new(sleep).await;
                        let mut st = self.state.lock().unwrap();
                        st.records.timeout_count += 1;
                        return fake_server::timeout_fault_xml();
                    }
                    Delay::new(wake_at.saturating_duration_since(now)).await;
                }
            }
        }
    }
}

// ── Metrics ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Metric {
    samples_ms: Vec<f64>,
}

impl Metric {
    fn push(&mut self, d: Duration) {
        self.samples_ms.push(d.as_secs_f64() * 1000.0);
    }

    fn summary(&self) -> String {
        if self.samples_ms.is_empty() {
            return "n=0".to_owned();
        }
        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = sorted[sorted.len() / 2];
        let max = sorted[sorted.len() - 1];
        let min = sorted[0];
        format!(
            "n={} min={min:.0}ms median={median:.0}ms max={max:.0}ms",
            sorted.len()
        )
    }

    fn json_fields(&self) -> String {
        if self.samples_ms.is_empty() {
            return r#""n":0"#.to_owned();
        }
        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        format!(
            r#""n":{},"min_ms":{:.1},"median_ms":{:.1},"max_ms":{:.1}"#,
            sorted.len(),
            sorted[0],
            sorted[sorted.len() / 2],
            sorted[sorted.len() - 1]
        )
    }
}

// ── Scenario driver ─────────────────────────────────────────────────────────

struct ScenarioResult {
    name: &'static str,
    output_latency: Metric,
    input_latency: Metric,
    kill_signal: Metric,
    total_requests: u64,
    receive_timeouts: u64,
    wall_ms: u64,
}

fn serial_config() -> ironposh_client_core::connector::WinRmConfig {
    let mut config = fake_server::test_config();
    // Match production serial mode (web + tokio client default).
    config.operation_timeout_secs = Some(0.25);
    config
}

/// Drive one scenario: fresh server, fresh session, run `actions`, collect metrics.
async fn run_scenario(
    name: &'static str,
    scripts: Vec<PipelineScript>,
    kill_after: Option<Duration>,
) -> ScenarioResult {
    let server = std::sync::Arc::new(FakeWinRmServer::new(scripts.clone()));
    let (client, host_io, mut session_events, task) = RemoteAsyncPowershellClient::open_task_serial(
        serial_config(),
        SharedServer(server.clone()),
    );

    let started = Instant::now();
    let server_for_driver = server.clone();

    let driver = async move {
        let mut client = client;
        let mut output_latency = Metric::default();
        let mut input_latency = Metric::default();
        let mut kill_signal = Metric::default();

        for _script in &scripts {
            let invoked_at = Instant::now();
            let mut events = client
                .send_script_raw("bench".to_owned())
                .await
                .expect("invoke pipeline");

            let mut handle = None;
            let mut kill_sent_at = None;

            loop {
                // Arm the kill timer only while the target pipeline is live.
                let event = if let (Some(kill_after), true, None) =
                    (kill_after, handle.is_some(), kill_sent_at)
                {
                    let due = invoked_at + kill_after;
                    let now = Instant::now();
                    if due > now {
                        futures::select! {
                            ev = events.next() => ev,
                            () = Delay::new(due - now).fuse() => {
                                kill_sent_at = Some(Instant::now());
                                let h = handle.take().expect("handle checked above");
                                client.kill_pipeline(h).await.expect("kill pipeline");
                                continue;
                            }
                        }
                    } else {
                        kill_sent_at = Some(Instant::now());
                        let h = handle.take().expect("handle checked above");
                        client.kill_pipeline(h).await.expect("kill pipeline");
                        continue;
                    }
                } else {
                    events.next().await
                };

                let Some(event) = event else { break };
                match event {
                    UserEvent::PipelineCreated { pipeline } => handle = Some(pipeline),
                    UserEvent::PipelineOutput { output, .. } => {
                        let payload = output
                            .assume_primitive_string()
                            .expect("bench outputs are strings")
                            .clone();
                        let ready_at = {
                            let st = server_for_driver.state.lock().unwrap();
                            st.records.ready_at[&payload]
                        };
                        output_latency.push(ready_at.elapsed());
                    }
                    UserEvent::PipelineFinished { .. } => break,
                    _ => {}
                }
            }

            // Input latency: Invoke → Command arrival, paired in order.
            {
                let st = server_for_driver.state.lock().unwrap();
                if let Some(arrived) = st.records.command_arrived_at.last() {
                    input_latency.push(*arrived - invoked_at);
                }
                if let (Some(sent), Some(arrived)) =
                    (kill_sent_at, st.records.signal_arrived_at.last())
                {
                    kill_signal.push(*arrived - sent);
                }
            }
        }

        drop(client);
        drop(host_io);
        (output_latency, input_latency, kill_signal)
    };

    let events_drain = async move { while session_events.next().await.is_some() {} };

    let (task_result, (output_latency, input_latency, kill_signal), ()) =
        join!(task, driver, events_drain);
    if let Err(e) = task_result {
        // Channel-closed on shutdown is the expected exit path; anything else
        // means the bench itself is broken.
        let msg = format!("{e:#}");
        assert!(
            msg.contains("channel closed") || msg.contains("channel disconnected"),
            "session task failed: {msg}"
        );
    }

    let st = server.state.lock().unwrap();
    ScenarioResult {
        name,
        output_latency,
        input_latency,
        kill_signal,
        total_requests: st.records.request_count,
        receive_timeouts: st.records.timeout_count,
        wall_ms: started.elapsed().as_millis() as u64,
    }
}

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("off")),
        )
        .init();

    let results = futures::executor::block_on(async {
        let mut results = Vec::new();

        // 1. REPL-style roundtrips: pipeline completes immediately.
        results.push(
            run_scenario(
                "prompt_roundtrip",
                (0..5)
                    .map(|i| PipelineScript {
                        outputs: vec![(secs(0.0), format!("pong-{i}"))],
                        complete_at: Some(secs(0.0)),
                    })
                    .collect(),
                None,
            )
            .await,
        );

        // 2. Steady drip: one line every 2s for 16s.
        results.push(
            run_scenario(
                "drip_2s",
                vec![PipelineScript {
                    outputs: (1..=8)
                        .map(|i| (secs(2.0 * i as f64), format!("drip-{i}")))
                        .collect(),
                    complete_at: Some(secs(16.5)),
                }],
                None,
            )
            .await,
        );

        // 3. Quiet then burst: silence long enough to reach max backoff, then
        // one line. Staggered quiet durations sample different phases of the
        // client-side backoff cycle (the worst case is a full backoff period).
        results.push(
            run_scenario(
                "quiet_burst_9_to_15s",
                [9.0, 10.5, 12.0, 13.5, 15.0]
                    .iter()
                    .enumerate()
                    .map(|(i, quiet)| PipelineScript {
                        outputs: vec![(secs(*quiet), format!("burst-{i}"))],
                        complete_at: Some(secs(*quiet)),
                    })
                    .collect(),
                None,
            )
            .await,
        );

        // 4. Ctrl+C during a quiet pipeline at +4s.
        results.push(
            run_scenario(
                "ctrl_c_at_4s",
                vec![PipelineScript {
                    outputs: vec![],
                    complete_at: None,
                }],
                Some(secs(4.0)),
            )
            .await,
        );

        results
    });

    println!("\n=== serial_latency_bench results ===");
    for r in &results {
        println!("\nscenario: {}", r.name);
        println!("  output_latency: {}", r.output_latency.summary());
        println!("  input_latency:  {}", r.input_latency.summary());
        println!("  kill_signal:    {}", r.kill_signal.summary());
        println!(
            "  requests={} receive_timeouts={} wall_ms={}",
            r.total_requests, r.receive_timeouts, r.wall_ms
        );
    }
    println!("\n=== json ===");
    for r in &results {
        println!(
            r#"{{"scenario":"{}","output":{{{}}},"input":{{{}}},"kill":{{{}}},"requests":{},"timeouts":{},"wall_ms":{}}}"#,
            r.name,
            r.output_latency.json_fields(),
            r.input_latency.json_fields(),
            r.kill_signal.json_fields(),
            r.total_requests,
            r.receive_timeouts,
            r.wall_ms
        );
    }
}
