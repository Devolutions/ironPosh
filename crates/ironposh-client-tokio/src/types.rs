use ironposh_terminal::TerminalOp;

/// UI operations for the async UI handler
#[derive(Debug, Clone)]
pub enum TerminalOperation {
    /// Apply terminal operations (cursor move, clear, fill, etc.)
    Apply(Vec<TerminalOp>),
    /// Print plain text lines
    Print(String),
    /// Request input from user with given prompt
    RequestInput { prompt: String },
    /// Check input for interrupt (Ctrl-C)
    CheckInterrupt,
}
