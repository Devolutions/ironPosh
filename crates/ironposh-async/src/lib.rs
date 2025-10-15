use futures::channel::mpsc;
use ironposh_client_core::connector::{conntion_pool::TrySend, http::HttpResponseTargeted};
use ironposh_client_core::host::{HostCall, HostCallScope, Submission};
use std::future::Future;

// Internal modules
mod connection;
mod session;

// Public API
pub mod client;

// Re-export the main client
pub use client::RemoteAsyncPowershellClient;

/// Session lifecycle events
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Connection process has started
    ConnectionStarted,
    /// Connection has been established successfully
    ConnectionEstablished,
    /// Active session loop has started
    ActiveSessionStarted,
    /// Active session loop has ended normally
    ActiveSessionEnded,
    /// An error occurred during connection or session
    Error(String),
    /// Session has been closed
    Closed,
}

/// Host I/O interface for handling PowerShell host calls
pub struct HostIo {
    /// Host calls coming from the runspace/pipelines
    pub host_call_rx: mpsc::UnboundedReceiver<HostCall>,
    /// Submits the host response back to the session
    pub submitter: HostSubmitter,
}

impl HostIo {
    /// Consume the HostIo and return the receiver and submitter separately
    pub fn into_parts(self) -> (mpsc::UnboundedReceiver<HostCall>, HostSubmitter) {
        (self.host_call_rx, self.submitter)
    }
}

/// Submitter for host call responses
#[derive(Clone)]
pub struct HostSubmitter(mpsc::UnboundedSender<HostResponse>);

/// Response to a host call
pub struct HostResponse {
    pub call_id: i64,
    pub scope: HostCallScope,
    pub submission: Submission,
}

impl HostSubmitter {
    /// Submit a host call response back to the session
    pub fn submit(&self, resp: HostResponse) -> anyhow::Result<()> {
        self.0
            .unbounded_send(resp)
            .map_err(|_| anyhow::anyhow!("Host response channel closed"))?;
        Ok(())
    }
}

pub trait AsyncPowershellClient {
    fn open_task(&self, client: impl HttpClient) -> impl Future<Output = anyhow::Result<()>>
    where
        Self: Sized;

    fn send_command(&self, command: String) -> impl Future<Output = anyhow::Result<String>>;
}

pub trait HttpClient: Send + Sync + 'static {
    fn send_request(
        &self,
        try_send: TrySend,
    ) -> impl Future<Output = anyhow::Result<HttpResponseTargeted>>;
}
