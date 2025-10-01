use anyhow::Context;
use futures::SinkExt;
use futures::channel::mpsc::Receiver;
use ironposh_client_core::connector::{WinRmConfig, active_session::UserEvent};
use ironposh_client_core::pipeline::{PipelineCommand, PipelineSpec};
use ironposh_client_core::powershell::PipelineHandle;
use tracing::instrument;

use crate::{
    HttpClient,
    connection::{self, ConnectionHandle},
};

/// Async PowerShell client for executing commands and managing sessions
#[derive(Clone)]
pub struct RemoteAsyncPowershellClient {
    handle: ConnectionHandle,
}

impl RemoteAsyncPowershellClient {
    /// Create a new client and background task for the given configuration
    /// Returns (client, host_io, connection_task)
    pub fn open_task(
        config: WinRmConfig,
        client: impl HttpClient,
    ) -> (Self, crate::HostIo, impl std::future::Future<Output = anyhow::Result<()>>)
    where
        Self: Sized,
    {
        let (handle, host_io, task) = connection::establish_connection(config, client);

        (Self { handle }, host_io, task)
    }

    /// Execute a PowerShell command and return its output
    #[instrument(skip(self))]
    pub async fn send_script(&mut self, script: String) -> anyhow::Result<Receiver<UserEvent>> {
        // Build the command pipeline
        let commands = vec![
            PipelineCommand::new_script(script),
            PipelineCommand::new_output_stream(),
        ];

        let (tx, rx) = futures::channel::mpsc::channel(10);

        self.handle
            .pipeline_input_tx
            .send(connection::PipelineInput::Invoke {
                uuid: uuid::Uuid::new_v4(),
                spec: PipelineSpec { commands },
                response_tx: tx,
            })
            .await
            .context("Failed to send CreatePipeline operation")?;

        Ok(rx)
    }

    #[instrument(skip(self))]
    pub async fn send_command(&mut self, command: String) -> anyhow::Result<Receiver<UserEvent>> {
        let (tx, rx) = futures::channel::mpsc::channel(10);

        self.handle
            .pipeline_input_tx
            .send(connection::PipelineInput::Invoke {
                uuid: uuid::Uuid::new_v4(),
                spec: PipelineSpec {
                    commands: vec![PipelineCommand::new_command(command)],
                },
                response_tx: tx,
            })
            .await
            .context("Failed to send CreatePipeline operation")?;

        Ok(rx)
    }

    pub async fn kill_pipeline(&mut self, pipeline_handle: PipelineHandle) -> anyhow::Result<()> {
        self.handle
            .pipeline_input_tx
            .send(connection::PipelineInput::Kill { pipeline_handle })
            .await
            .context("Failed to send KillPipeline operation")?;

        Ok(())
    }
}
