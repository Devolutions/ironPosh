use std::fmt;

use ironposh_client_core::connector::http::HttpResponseTargeted;
use ironposh_client_core::connector::UserOperation;
use ironposh_terminal::TerminalOp;

/// Represents the next step in the event loop
#[derive(Debug)]
pub enum NextStep {
    NetworkResponse(HttpResponseTargeted),
    UserRequest(Box<UserOperation>),
}

impl fmt::Display for NextStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NextStep::NetworkResponse(_) => write!(f, "NetworkResponse"),
            NextStep::UserRequest(_) => write!(f, "UserRequest"),
        }
    }
}

/// UI operations for the dedicated UI thread
#[derive(Debug)]
pub enum UiOp {
    Apply(Vec<TerminalOp>), // render ops (cursor move, clear, fill, bytes…)
    Print(String),          // for plain text lines if you want
}
