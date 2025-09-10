use std::fmt;

use ironposh_client_core::connector::UserOperation;

/// Represents the next step in the event loop
#[derive(Debug)]
pub enum NextStep {
    NetworkResponse(ironposh_client_core::connector::http::HttpResponse),
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
