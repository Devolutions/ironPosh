//! Serial session loop — single-connection mode for Devolutions Gateway.
//!
//! Split into two layers:
//! - [`core::SessionCore`] — pure synchronous protocol decisions (queues, promotion, routing)
//! - [`start_serial_session_loop`] — thin async I/O shell (HTTP, channels, `select!`)

mod core;

use anyhow::Context;
use futures::channel::mpsc;
use futures::future::Either;
use futures::{FutureExt, SinkExt, StreamExt};
use ironposh_client_core::connector::active_session::{ActiveSession, UserEvent};
use ironposh_client_core::connector::conntion_pool::TrySend;
use ironposh_client_core::host::HostCall;
use tracing::{info, instrument, trace};

use ironposh_client_core::connector::UserOperation;

use self::core::SessionCore;
use crate::{HostResponse, HttpClient};

/// Console diagnostic logging for WASM debugging.
#[cfg(all(feature = "wasm-diag", target_arch = "wasm32"))]
macro_rules! diag {
    ($($arg:tt)*) => {
        web_sys::console::error_1(&format!($($arg)*).into());
    };
}
#[cfg(not(all(feature = "wasm-diag", target_arch = "wasm32")))]
macro_rules! diag {
    ($($arg:tt)*) => {};
}
pub(in crate::session_serial) use diag;

/// Serial active session loop using a flat `select!`-based event loop.
///
/// **Core invariant:** At most one HTTP request is in flight at any time.
/// All protocol decisions (promotion priority, HostCall blocking, speculative
/// vs demanded Receives) are handled by [`SessionCore`].
#[instrument(skip_all)]
pub async fn start_serial_session_loop(
    first_receive: TrySend,
    active_session: ActiveSession,
    client: impl HttpClient,
    mut user_input_rx: mpsc::Receiver<UserOperation>,
    mut user_output_tx: mpsc::Sender<UserEvent>,
    host_call_tx: mpsc::UnboundedSender<HostCall>,
    mut host_resp_rx: mpsc::UnboundedReceiver<HostResponse>,
) -> anyhow::Result<()> {
    let mut core = SessionCore::new(first_receive, active_session);

    info!("Starting serial session loop (flat event loop, single-connection mode)");
    diag!("DIAG serial loop: started (flat event loop)");

    loop {
        // Dispatch accumulated effects from the previous iteration.
        dispatch_effects(&mut core, &mut user_output_tx, &host_call_tx).await?;

        // Process one buffered user op if connection is idle.
        core.process_one_buffered_op()?;

        if let Some(req) = core.promote_next_request()? {
            // HTTP in-flight: send request and buffer incoming ops until response.
            let resp = send_and_buffer(
                &client,
                req,
                &mut core,
                &mut user_input_rx,
                &mut host_resp_rx,
                &host_call_tx,
            )
            .await?;

            core.accept_response(resp)?;

            // Drain any user ops that arrived while HTTP was in flight.
            if drain_channel(&mut core, &mut user_input_rx) {
                return Ok(()); // channel closed
            }
        } else {
            // Idle — wait for user op or host response.
            trace!(target: "serial", "idle: no pending work, waiting for user input or host response");
            diag!("DIAG idle: waiting for user input or host response");

            let mut host_guard = if core.is_host_call_active() {
                Either::Left(host_resp_rx.next())
            } else {
                Either::Right(futures::future::pending::<Option<HostResponse>>())
            };

            futures::select! {
                op = user_input_rx.next() => {
                    if let Some(op) = op {
                        core.accept_user_op(op)?;
                    } else {
                        info!("User input channel closed (idle), ending serial session loop");
                        return Ok(());
                    }
                }
                hr = host_guard => {
                    if let Some(hr) = hr {
                        core.accept_host_response(hr)?;
                    } else {
                        return Err(anyhow::anyhow!("Host-response channel closed (idle)"));
                    }
                }
            }
        }
    }
}

/// Send an HTTP request and buffer incoming user ops / host responses until
/// the response arrives.
async fn send_and_buffer(
    client: &impl HttpClient,
    req: TrySend,
    core: &mut SessionCore,
    user_input_rx: &mut mpsc::Receiver<UserOperation>,
    host_resp_rx: &mut mpsc::UnboundedReceiver<HostResponse>,
    host_call_tx: &mpsc::UnboundedSender<HostCall>,
) -> anyhow::Result<crate::HttpResponseTargeted> {
    let http_future = client.send_request(req).fuse();
    futures::pin_mut!(http_future);

    loop {
        let mut host_guard = if core.is_host_call_active() {
            Either::Left(host_resp_rx.next())
        } else {
            Either::Right(futures::future::pending::<Option<HostResponse>>())
        };

        futures::select! {
            resp = http_future => {
                diag!("DIAG select: HTTP response received");
                trace!(target: "serial", "HTTP response received");
                return resp.context("Serial HTTP request failed");
            }
            op = user_input_rx.next() => {
                if let Some(op) = op {
                    core.buffer_user_op(op);
                } else {
                    info!("User input channel closed, ending serial session loop");
                    return Err(anyhow::anyhow!("User input channel closed during HTTP wait"));
                }
            }
            hr = host_guard => {
                match hr {
                    Some(hr) => {
                        core.buffer_host_response(hr);
                        // Dispatch next HostCall immediately (just a channel send, no HTTP).
                        while let Some(hc) = core.poll_host_call() {
                            if host_call_tx.unbounded_send(hc).is_err() {
                                return Err(anyhow::anyhow!("Host-call channel closed"));
                            }
                        }
                    }
                    None => return Err(anyhow::anyhow!("Host-response channel closed")),
                }
            }
        }
    }
}

/// Drain buffered user ops from the channel (after HTTP response).
/// Returns `true` if the channel is closed.
fn drain_channel(
    core: &mut SessionCore,
    user_input_rx: &mut mpsc::Receiver<UserOperation>,
) -> bool {
    loop {
        match user_input_rx.try_next() {
            Ok(Some(op)) => {
                diag!("DIAG drain: collected {}", op.operation_type());
                core.buffer_user_op(op);
            }
            Ok(None) => {
                info!("User input channel closed during drain");
                return true;
            }
            Err(_) => return false,
        }
    }
}

/// Dispatch accumulated user events and host calls to their channels.
async fn dispatch_effects(
    core: &mut SessionCore,
    user_output_tx: &mut mpsc::Sender<UserEvent>,
    host_call_tx: &mpsc::UnboundedSender<HostCall>,
) -> anyhow::Result<()> {
    for event in core.drain_user_events() {
        diag!("DIAG dispatch: UserEvent");
        if user_output_tx.send(event).await.is_err() {
            return Err(anyhow::anyhow!("User output channel disconnected"));
        }
    }
    while let Some(hc) = core.poll_host_call() {
        if host_call_tx.unbounded_send(hc).is_err() {
            return Err(anyhow::anyhow!("Host-call channel closed"));
        }
    }
    Ok(())
}
