use anyhow::Context;
use futures::{SinkExt, StreamExt};
use ironposh_client_core::connector::{UserOperation, WinRmConfig, active_session::UserEvent};
use ironposh_client_core::pipeline::{Parameter, PipelineCommand, PipelineSpec};
use tracing::{info, instrument};

use crate::{
    HttpClient,
    connection::{self, ConnectionHandle},
};

/// Async PowerShell client for executing commands and managing sessions
pub struct RemoteAsyncPowershellClient {
    handle: ConnectionHandle,
}

impl RemoteAsyncPowershellClient {
    /// Create a new client and background task for the given configuration
    pub fn open_task(
        config: WinRmConfig,
        client: impl HttpClient,
    ) -> (Self, impl std::future::Future<Output = anyhow::Result<()>>)
    where
        Self: Sized,
    {
        let (handle, task) = connection::establish_connection(config, client);

        (Self { handle }, task)
    }

    /// Execute a PowerShell command and return its output
    #[instrument(skip(self))]
    pub async fn send_command(
        &mut self,
        command: String,
        new_line: bool,
    ) -> anyhow::Result<String> {
        let new_pipeline_id = uuid::Uuid::new_v4();

        // Build the command pipeline
        let mut commands = vec![
            PipelineCommand::new_command("Invoke-Expression".to_string()).with_parameter(
                Parameter::Named {
                    name: "Command".to_string(),
                    value: command.into(),
                },
            ),
        ];

        // Add Out-String with appropriate parameters
        if new_line {
            commands.push(
                PipelineCommand::new_command("Out-String".to_string()).with_parameter(
                    Parameter::Switch {
                        name: "Stream".to_string(),
                        value: true,
                    },
                ),
            );
        } else {
            commands.push(PipelineCommand::new_command("Out-String".to_string()));
        }

        // Send the single invoke operation
        self.handle
            .user_input_tx
            .send(UserOperation::InvokeWithSpec {
                uuid: new_pipeline_id,
                spec: PipelineSpec { commands },
            })
            .await
            .context("Failed to send invoke with spec operation")?;

        let mut pipeline_ended = false;
        let mut result = String::new();

        while !pipeline_ended {
            let events = self.receive_from_pipeline(new_pipeline_id).await?;
            info!(pipeline_id = %new_pipeline_id, event_count = events.len(), "received events from pipeline");
            for event in events {
                match event {
                    UserEvent::PipelineOutput { output, pipeline } => {
                        debug_assert!(pipeline.id() == new_pipeline_id);
                        info!(pipeline_id = %new_pipeline_id, output = ?output, "received pipeline output");
                        result.push_str(&output.format_as_displyable_string()?);
                    }
                    UserEvent::PipelineFinished { pipeline } => {
                        debug_assert!(pipeline.id() == new_pipeline_id);
                        info!(pipeline_id = %new_pipeline_id, "pipeline finished");
                        pipeline_ended = true;
                    }
                    UserEvent::PipelineCreated { .. } => {
                        // Ignore creation events in the new API
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get the current PowerShell prompt
    #[instrument(skip(self))]
    pub async fn prompt(&mut self) -> anyhow::Result<String> {
        let result = self.send_command("prompt".to_string(), false).await?;
        Ok(result.trim_end().to_string())
    }

    /// Receive events from a specific pipeline, handling message caching
    #[instrument(skip(self))]
    async fn receive_from_pipeline(
        &mut self,
        pipeline_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<UserEvent>> {
        if let Some(events) = self.handle.message_cache.remove(&pipeline_id) {
            info!(pipeline_id = %pipeline_id, cached_event_count = events.len(), "returning cached events");
            return Ok(events);
        }

        loop {
            if let Some(event) = self.handle.user_output_rx.next().await {
                info!(?event, "received user event");
                if event.pipeline_id() == pipeline_id {
                    return Ok(vec![event]);
                } else {
                    self.handle
                        .message_cache
                        .entry(event.pipeline_id())
                        .or_default()
                        .push(event);
                }
            }
        }
    }
}
