/// A handle to a PowerShell pipeline managed by a `RunspacePool`.
///
/// This struct is a lightweight, copyable identifier for a specific pipeline.
/// All operations on the pipeline are performed via methods on the `RunspacePool`
/// that take this handle as an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle {
    pub(crate) id: uuid::Uuid,
}

impl PipelineHandle {
    /// Creates a PowerShell handle from a server-provided UUID.
    ///
    /// This function is internal because handles should only be created by
    /// the server via the PSRP protocol flow.
    pub(crate) fn from_server_id(id: uuid::Uuid) -> Self {
        Self { id }
    }

    /// Returns the unique identifier for this PowerShell handle.
    pub fn id(&self) -> uuid::Uuid {
        self.id
    }
}
