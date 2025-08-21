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
}

/// Defines how the output of a pipeline should be handled
/// This concept is not part of the PWSH protocol, it is used internally
/// to determine how the output should be processed when invoking a pipeline.
#[derive(Debug)]
pub enum PipelineOutputType {
    Raw,
    /// Invoke pipeline with a extra command `Out-String -Stream`
    Streamed,
}
