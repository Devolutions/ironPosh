use uuid::Uuid;

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
    /// Returns the unique identifier for this PowerShell handle.
    pub fn id(&self) -> uuid::Uuid {
        self.id
    }

    pub fn new(id: Uuid) -> Self {
        Self { id }
    }
}
