use anyhow::Context;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
use ironposh_client_core::connector::active_session::UserEvent;

use ironposh_client_core::connector::{
    Connector, ConnectorStepResult, UserOperation,
    http::{HttpRequest, HttpResponse},
};
use ironposh_client_core::pipeline::Parameter;
use tracing::{Instrument, debug, error, info, info_span, instrument, warn};

use crate::HttpClient;

fn launch<C: HttpClient>(
    client: &C,
    req: HttpRequest,
) -> impl core::future::Future<Output = anyhow::Result<HttpResponse>> + Send {
    client.send_request(req)
}

pub struct RemoteAsyncPowershellClient {
    user_input_tx: mpsc::Sender<UserOperation>,
    user_output_rx: mpsc::Receiver<ironposh_client_core::connector::active_session::UserEvent>,
    message_cache: std::collections::HashMap<uuid::Uuid, Vec<UserEvent>>,
}

impl RemoteAsyncPowershellClient {
    #[instrument(skip_all)]
    async fn start_active_session_loop(
        runspace_polling_request: ironposh_client_core::connector::http::HttpRequest,
        mut active_session: ironposh_client_core::connector::active_session::ActiveSession,
        client: impl HttpClient,
        mut user_input_rx: mpsc::Receiver<ironposh_client_core::connector::UserOperation>,
        mut user_output_tx: mpsc::Sender<
            ironposh_client_core::connector::active_session::UserEvent,
        >,
        mut user_input_tx: mpsc::Sender<ironposh_client_core::connector::UserOperation>,
    ) -> anyhow::Result<()> {
        use ironposh_client_core::connector::active_session::ActiveSessionOutput;
        use tracing::{error, info};

        // pending HTTP requests
        let mut inflight: FuturesUnordered<_> = FuturesUnordered::new();

        // kick off the initial polling request
        let initial_poll_req = runspace_polling_request.clone();
        inflight.push(launch(&client, initial_poll_req));

        info!("Starting single-loop active session");

        // main single-threaded loop
        loop {
            futures::select! {
                // 1) any HTTP finishes
                ready = inflight.select_next_some() => {
                    match ready {
                        Ok(http_response) => {
                            info!(
                                target: "network",
                                body_length = http_response.body.as_ref().map(|b| b.len()).unwrap_or(0),
                                "processing successful network response"
                            );

                            let step_results = active_session
                                .accept_server_response(http_response)
                                .map_err(|e| {
                                    error!(target: "network", error = %e, "failed to accept server response");
                                    e
                                })
                                .context("Failed to accept server response")?;

                            // Convert ActiveSessionOutput into new HTTPs / UI events
                            for out in step_results {
                                match out {
                                    ActiveSessionOutput::SendBack(reqs) => {
                                        info!(
                                            target: "network",
                                            request_count = reqs.len(),
                                            "launching HTTP requests in parallel"
                                        );
                                        // launch all new HTTPs in parallel
                                        for r in reqs {
                                            inflight.push(launch(&client, r));
                                        }
                                    }
                                    ActiveSessionOutput::SendBackError(e) => {
                                        error!(target: "session", error = %e, "session step failed");
                                        return Err(anyhow::anyhow!("Session step failed: {}", e));
                                    }
                                    _ => {
                                        // unchanged: fan out to UI or user_input
                                        Self::process_session_outputs(vec![out], &mut user_output_tx, &mut user_input_tx).await?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Any HTTP error terminates the session
                            error!(target: "network", error = %e, "HTTP request failed");
                            return Err(anyhow::anyhow!("HTTP error: {e:#}"));
                        }
                    }
                }

                // 2) user operations
                user_op = user_input_rx.next() => {
                    info!(target: "user", "processing user operation");
                    match user_op {
                        Some(user_operation) => {
                            info!(target: "user", operation = ?user_operation, "processing user operation");

                            let step_result = active_session
                                .accept_client_operation(user_operation)
                                .map_err(|e| {
                                    error!(target: "user", error = %e, "failed to accept user operation");
                                    e
                                })
                                .context("Failed to accept user operation")?;

                            match step_result {
                                ActiveSessionOutput::SendBack(reqs) => {
                                    info!(
                                        target: "network",
                                        request_count = reqs.len(),
                                        "launching HTTP requests from user operation"
                                    );
                                    for r in reqs {
                                        inflight.push(launch(&client, r));
                                    }
                                }
                                _ => Self::process_session_outputs(vec![step_result], &mut user_output_tx, &mut user_input_tx).await?,
                            }
                        }
                        None => {
                            info!("User input channel disconnected");
                            break; // UI side closed
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn process_session_outputs(
        step_results: Vec<ironposh_client_core::connector::active_session::ActiveSessionOutput>,
        user_output_tx: &mut mpsc::Sender<
            ironposh_client_core::connector::active_session::UserEvent,
        >,
        user_input_tx: &mut mpsc::Sender<UserOperation>,
    ) -> anyhow::Result<()> {
        use ironposh_client_core::connector::active_session::ActiveSessionOutput;
        use ironposh_client_core::host::{HostCallMethodReturn, RawUIMethodReturn};
        use tracing::{error, info, warn};

        for step_result in step_results {
            info!(step_result = ?step_result, "processing step result");

            match step_result {
                ActiveSessionOutput::SendBack(_) => {
                    // SendBack is now handled directly in the main loop
                    warn!("SendBack should not be passed to process_session_outputs anymore");
                }
                ActiveSessionOutput::SendBackError(e) => {
                    error!(target: "session", error = %e, "session step failed");
                    return Err(anyhow::anyhow!("Session step failed: {}", e));
                }
                ActiveSessionOutput::UserEvent(event) => {
                    info!(target: "user", event = ?event, "sending user event");
                    if let Err(e) = user_output_tx.send(event).await {
                        error!(target: "user", error = %e, "failed to send user event");
                    }
                }
                /*
                    This is the complex part - handling host calls
                    TODO: Implement more host call methods as needed
                */
                ActiveSessionOutput::HostCall(host_call) => {
                    info!(
                        target: "host",
                        method_name = %host_call.method_name,
                        call_id = host_call.call_id,
                        "received host call"
                    );

                    let method = host_call.get_param().map_err(|e| {
                        error!(target: "host", error = %e, "failed to parse host call parameters");
                        e
                    })?;

                    info!(target: "host", method = ?method, "processing host call method");

                    let response = match method {
                        ironposh_client_core::host::HostCallMethodWithParams::RawUIMethod(
                            ironposh_client_core::host::RawUIMethodParams::GetBufferSize,
                        ) => {
                            info!(target: "host", method = "GetBufferSize", "returning default console size");
                            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetBufferSize(
                                120, 30,
                            ))
                        }
                        ironposh_client_core::host::HostCallMethodWithParams::UIMethod(
                            ironposh_client_core::host::UIMethodParams::WriteProgress(
                                source_id,
                                record,
                            ),
                        ) => {
                            info!(
                                target: "host",
                                method = "WriteProgress",
                                source_id = source_id,
                                record = %record,
                                "handling write progress"
                            );
                            HostCallMethodReturn::UIMethod(
                                ironposh_client_core::host::UIMethodReturn::WriteProgress,
                            )
                        }
                        other => {
                            warn!(target: "host", method = ?other, "host call method not implemented");
                            HostCallMethodReturn::Error(
                                ironposh_client_core::host::HostError::NotImplemented,
                            )
                        }
                    };

                    let host_response = host_call.submit_result(response);
                    info!(
                        target: "host",
                        call_id = host_response.call_id,
                        "created host call response"
                    );

                    let user_event = UserOperation::SubmitHostResponse {
                        response: Box::new(host_response),
                    };

                    user_input_tx
                        .send(user_event)
                        .await
                        .map_err(|e| {
                            error!(target: "host", error = %e, "failed to send host response to user input");
                            e
                        })
                        .context("Failed to send host response to user input")?;
                }
                ActiveSessionOutput::OperationSuccess => {
                    info!(target: "session", "operation completed successfully");
                }
            }
        }

        Ok(())
    }
}

impl RemoteAsyncPowershellClient {
    pub fn open_task(
        config: ironposh_client_core::connector::WinRmConfig,
        client: impl HttpClient,
    ) -> (Self, impl std::future::Future<Output = anyhow::Result<()>>)
    where
        Self: Sized,
    {
        let (user_input_tx, user_input_rx) = mpsc::channel(10);
        let (user_output_tx, user_output_rx) = mpsc::channel(10);

        let user_input_tx_clone = user_input_tx.clone();
        let task = async move {
            let mut connector = Connector::new(config);
            info!("Created connector, starting connection...");

            let mut response = None;

            let (active_session, next_request) = loop {
                let step_result = connector
                    .step(response.take())
                    .context("Failed to step through connector")?;

                info!(step_result = ?step_result.name(), "Processing step result");

                match step_result {
                    ConnectorStepResult::SendBack(http_request) => {
                        // Make the HTTP request (using ureq for simplicity in example)
                        response = Some(client.send_request(http_request).await?);
                    }
                    ConnectorStepResult::SendBackError(e) => {
                        warn!("Connection step failed: {}", e);
                        anyhow::bail!("Connection failed: {}", e);
                    }
                    ConnectorStepResult::Connected {
                        active_session,
                        send_this_one_async_or_you_stuck: next_receive_request,
                    } => {
                        break (active_session, next_receive_request);
                    }
                    ConnectorStepResult::Auth { sequence: _ } => {
                        info!("Starting authentication sequence");
                        // TODO: Fix this pattern - need proper implementation
                        todo!("Fix auth sequence handling");
                    }
                }
            };

            info!("Connection established, entering active session loop");
            Self::start_active_session_loop(
                next_request,
                *active_session,
                client,
                user_input_rx,
                user_output_tx,
                user_input_tx_clone,
            )
            .instrument(info_span!("ActiveSession"))
            .await?;

            info!("Active session loop ended");

            Ok(())
        }
        .instrument(info_span!("MainTask"));

        (
            Self {
                user_input_tx,
                user_output_rx,
                message_cache: std::collections::HashMap::new(),
            },
            task,
        )
    }

    #[instrument(skip(self))]
    pub async fn send_command(
        &mut self,
        command: String,
        new_line: bool,
    ) -> anyhow::Result<String> {
        let new_pipeline_id = uuid::Uuid::new_v4();

        self.user_input_tx
            .send(UserOperation::CreatePipeline {
                uuid: new_pipeline_id,
            })
            .await
            .context("Failed to send create pipeline operation")?;

        debug!(pipeline_id = %new_pipeline_id, "waiting for pipeline output");
        let powershell = 'outer: loop {
            let events = self.receive_from_pipeline(new_pipeline_id).await?;
            info!(pipeline_id = %new_pipeline_id, event_count = events.len(), "received events from pipeline");
            for event in events {
                if let UserEvent::PipelineCreated { powershell } = event {
                    // Definatly the same, just check to be sure
                    debug_assert!(powershell.id() == new_pipeline_id);
                    break 'outer powershell;
                }
            }
        };
        debug!(pipeline_id = %new_pipeline_id, "pipeline created, sending command");

        self.user_input_tx
            .send(
                powershell
                    .command_builder("Invoke-Expression".to_string())
                    .with_param(Parameter::Named {
                        name: "Command".to_string(),
                        value: command.into(),
                    })
                    .build(),
            )
            .await
            .context("Failed to send add command operation")?;

        let builder = powershell.command_builder("Out-String".to_string());
        let out_string = if new_line {
            builder
                .with_param(Parameter::Switch {
                    name: "Stream".to_string(),
                    value: true,
                })
                .build()
        } else {
            builder.build()
        };

        self.user_input_tx
            .send(out_string)
            .await
            .context("Failed to send invoke pipeline operation")?;

        self.user_input_tx
            .send(powershell.invoke())
            .await
            .context("Failed to send invoke pipeline operation")?;

        let mut pipeline_ended = false;
        let mut result = String::new();

        while !pipeline_ended {
            let events = self.receive_from_pipeline(new_pipeline_id).await?;
            info!(pipeline_id = %new_pipeline_id, event_count = events.len(), "received events from pipeline");
            for event in events {
                match event {
                    UserEvent::PipelineOutput { output, powershell } => {
                        debug_assert!(powershell.id() == new_pipeline_id);
                        info!(pipeline_id = %new_pipeline_id, output = ?output, "received pipeline output");
                        result.push_str(&output.format_as_ps_string()?);
                    }
                    UserEvent::PipelineFinished { powershell } => {
                        debug_assert!(powershell.id() == new_pipeline_id);
                        info!(pipeline_id = %new_pipeline_id, "pipeline finished");
                        pipeline_ended = true;
                    }
                    other => {
                        warn!(pipeline_id = %new_pipeline_id, event = ?other, "unexpected event received");
                    }
                }
            }
        }

        Ok(result)
    }

    #[instrument(skip(self))]
    pub async fn prompt(&mut self) -> anyhow::Result<String> {
        let result = self.send_command("prompt".to_string(), false).await?;
        Ok(result.trim_end().to_string())
    }

    #[instrument(skip(self))]
    async fn receive_from_pipeline(
        &mut self,
        pipeline_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<UserEvent>> {
        if let Some(events) = self.message_cache.remove(&pipeline_id) {
            info!(pipeline_id = %pipeline_id, cached_event_count = events.len(), "returning cached events");
            return Ok(events);
        }

        loop {
            if let Some(event) = self.user_output_rx.next().await {
                info!(?event, "received user event");
                if event.pipeline_id() == pipeline_id {
                    return Ok(vec![event]);
                } else {
                    self.message_cache
                        .entry(event.pipeline_id())
                        .or_default()
                        .push(event);
                }
            }
        }
    }
}
