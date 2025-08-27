use anyhow::Context;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use pwsh_core::connector::active_session::UserEvent;
use pwsh_core::{
    connector::{Connector, ConnectorStepResult, UserOperation, http::HttpRequest},
    pipeline::PipelineCommand,
};
use tracing::{Instrument, debug, error, info, info_span, instrument, warn};

use crate::HttpClient;

pub struct RemoteAsyncPowershellClient {
    user_input_tx: mpsc::Sender<UserOperation>,
    user_output_rx: mpsc::Receiver<pwsh_core::connector::active_session::UserEvent>,
    message_cache: std::collections::HashMap<uuid::Uuid, Vec<UserEvent>>,
}

impl RemoteAsyncPowershellClient {
    #[instrument(skip_all)]
    async fn start_active_session_loop(
        runspace_polling_request: pwsh_core::connector::http::HttpRequest<String>,
        mut active_session: pwsh_core::connector::active_session::ActiveSession,
        client: impl HttpClient,
        mut user_input_rx: mpsc::Receiver<pwsh_core::connector::UserOperation>,
        mut user_output_tx: mpsc::Sender<pwsh_core::connector::active_session::UserEvent>,
        mut user_input_tx: mpsc::Sender<pwsh_core::connector::UserOperation>,
    ) -> anyhow::Result<()> {
        use tracing::{error, info};

        // Create channels for network request/response handling
        let (mut network_request_tx, mut network_request_rx) = mpsc::channel(10);
        let (network_response_tx, mut network_response_rx) = mpsc::channel(10);

        info!("Starting Network task and Active Session task");
        network_request_tx
            .send(runspace_polling_request)
            .await
            .context("Failed to send initial runspace polling request")?;

        // Network task - handles HTTP requests/responses
        info!("Starting Network task");
        let network_task = async move {
            info!("Network task started");
            while let Some(http_request) = network_request_rx.next().await {
                info!(
                    target: "network",
                    method = ?http_request.method,
                    url = %http_request.url,
                    headers_count = http_request.headers.len(),
                    body_length = http_request.body.as_ref().map(|b| b.len()).unwrap_or(0),
                    "sending network request"
                );

                let mut nextwork_response_tx_clone = network_response_tx.clone();
                client.send_request_callback(http_request, move |result| {
                    info!(target: "network", "received network response callback");
                    // TODO: Handle error properly
                    let Ok(result) = result else {
                        error!(target: "network", error = %result.unwrap_err(), "HTTP request failed");
                        return;
                    };
                    if let Err(e) = nextwork_response_tx_clone.try_send(result) {
                        error!(target: "network", error = %e, "failed to send network response");
                    }
                });
            }

            anyhow::Ok(())
        }
        .instrument(info_span!("NetworkTask"));

        info!("Starting Active Session task");
        // Session task - handles the main event loop
        let session_task = async move {
            info!("Active Session task started");
            loop {
                // Handle both network responses and user requests like the sync version
                futures::select! {
                    network_response = network_response_rx.next() => {
                        info!(target: "network", "processing network response");
                        match network_response {
                            Some(http_response) => {
                                info!(
                                    target: "network",
                                    body_length = http_response.body.as_ref().map(|b| b.len()).unwrap_or(0),
                                    "processing network response"
                                );

                                let step_results = active_session
                                    .accept_server_response(http_response)
                                    .map_err(|e| {
                                        error!(target: "network", error = %e, "failed to accept server response");
                                        e
                                    })
                                    .context("Failed to accept server response")?;

                                Self::process_session_outputs(step_results, &mut network_request_tx, &mut user_output_tx, &mut user_input_tx).await?;
                            }
                            None => {
                                error!("Network response channel disconnected");
                                return Err(anyhow::anyhow!("Network response channel disconnected"));
                            }
                        }
                    }
                    user_operation = user_input_rx.next() => {
                        info!(target: "user", "processing user operation");
                        match user_operation {
                            Some(user_operation) => {
                                info!(target: "user", operation = ?user_operation, "processing user operation");

                                let step_result = active_session
                                    .accept_client_operation(user_operation)
                                    .map_err(|e| {
                                        error!(target: "user", error = %e, "failed to accept user operation");
                                        e
                                    })
                                    .context("Failed to accept user operation")?;

                                Self::process_session_outputs(vec![step_result], &mut network_request_tx, &mut user_output_tx, &mut user_input_tx).await?;
                            }
                            None => {
                                info!("User input channel disconnected");
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }.instrument(info_span!("ActiveSessionTask"));

        // Use futures::join! to run both tasks concurrently
        let (session_result, network_result) = futures::join!(session_task, network_task);
        session_result.and(network_result)
    }

    #[instrument(skip_all)]
    async fn process_session_outputs(
        step_results: Vec<pwsh_core::connector::active_session::ActiveSessionOutput>,
        network_request_tx: &mut mpsc::Sender<HttpRequest<String>>,
        user_output_tx: &mut mpsc::Sender<pwsh_core::connector::active_session::UserEvent>,
        user_input_tx: &mut mpsc::Sender<UserOperation>,
    ) -> anyhow::Result<()> {
        use pwsh_core::connector::active_session::ActiveSessionOutput;
        use pwsh_core::host::{HostCallMethodReturn, RawUIMethodReturn};
        use tracing::{error, info, warn};

        for step_result in step_results {
            info!(step_result = ?step_result, "processing step result");

            match step_result {
                ActiveSessionOutput::SendBack(http_requests) => {
                    info!(
                        target: "network",
                        request_count = http_requests.len(),
                        "sending HTTP requests to network task"
                    );
                    for http_request in http_requests {
                        if let Err(e) = network_request_tx.send(http_request).await {
                            error!(target: "network", error = %e, "failed to send HTTP request to network task");
                            return Err(anyhow::anyhow!(
                                "Failed to send HTTP request to network task: {}",
                                e
                            ));
                        }
                    }
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
                        pwsh_core::host::HostCallMethodWithParams::RawUIMethod(
                            pwsh_core::host::RawUIMethodParams::GetBufferSize,
                        ) => {
                            info!(target: "host", method = "GetBufferSize", "returning default console size");
                            HostCallMethodReturn::RawUIMethod(RawUIMethodReturn::GetBufferSize(
                                120, 30,
                            ))
                        }
                        pwsh_core::host::HostCallMethodWithParams::UIMethod(
                            pwsh_core::host::UIMethodParams::WriteProgress(source_id, record),
                        ) => {
                            info!(
                                target: "host",
                                method = "WriteProgress",
                                source_id = source_id,
                                record = %record,
                                "handling write progress"
                            );
                            HostCallMethodReturn::UIMethod(
                                pwsh_core::host::UIMethodReturn::WriteProgress,
                            )
                        }
                        other => {
                            warn!(target: "host", method = ?other, "host call method not implemented");
                            HostCallMethodReturn::Error(pwsh_core::host::HostError::NotImplemented)
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
        config: pwsh_core::connector::ConnectorConfig,
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
                        next_receive_request,
                    } => {
                        break (active_session, next_receive_request);
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
    pub async fn send_command(&mut self, command: String) -> anyhow::Result<String> {
        let new_pipeline_id = uuid::Uuid::new_v4();

        self.user_input_tx
            .send(UserOperation::CreatePipeline {
                uuid: new_pipeline_id,
            })
            .await
            .context("Failed to send create pipeline operation")?;

        debug!(pipeline_id = %new_pipeline_id, "waiting for pipeline output");
        'outer: loop {
            let events = self.receive_from_pipeline(new_pipeline_id).await?;
            info!(pipeline_id = %new_pipeline_id, event_count = events.len(), "received events from pipeline");
            for event in events {
                if let UserEvent::PipelineCreated { powershell } = &event {
                    // Definatly the same, just check to be sure
                    debug_assert!(powershell.id() == new_pipeline_id);
                    break 'outer;
                }
            }
        }
        debug!(pipeline_id = %new_pipeline_id, "pipeline created, sending command");

        self.user_input_tx
            .send(UserOperation::OperatePipeline {
                powershell: pwsh_core::powershell::PipelineHandle::new(new_pipeline_id),
                operation: pwsh_core::connector::active_session::PowershellOperations::AddCommand {
                    command: PipelineCommand::new_script(command),
                },
            })
            .await
            .context("Failed to send add command operation")?;

        self.user_input_tx
            .send(UserOperation::InvokePipeline {
                powershell: pwsh_core::powershell::PipelineHandle::new(new_pipeline_id),
                output_type: pwsh_core::powershell::PipelineOutputType::Streamed,
            })
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
