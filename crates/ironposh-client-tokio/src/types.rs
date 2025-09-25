use std::fmt;

use ironposh_client_core::connector::active_session::UserEvent;
use ironposh_terminal::TerminalOp;

/// UI operations for the async UI handler
#[derive(Debug, Clone)]
pub enum UiOp {
    /// Apply terminal operations (cursor move, clear, fill, etc.)
    Apply(Vec<TerminalOp>),
    /// Print plain text lines
    Print(String),
}

/// Unified input event for the async UI loop
#[derive(Debug)]
pub enum UIInputEvent {
    /// UI operation (rendering, printing)
    UiOp(UiOp),
    /// User event from PowerShell pipeline
    UserEvent(UserEvent),
}

impl fmt::Display for UIInputEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UIInputEvent::UiOp(_) => write!(f, "UiOp"),
            UIInputEvent::UserEvent(_) => write!(f, "UserEvent"),
        }
    }
}
