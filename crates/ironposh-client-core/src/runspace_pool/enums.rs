use ironposh_psrp::{PSInvocationState, RunspacePoolStateValue};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PowerShellState {
    CreatePipelineSent,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsInvocationState {
    NotStarted = 0,
    Running = 1,
    Stopping = 2,
    Stopped = 3,
    Completed = 4,
    Failed = 5,
    Disconnected = 6,
}

impl From<PSInvocationState> for PsInvocationState {
    fn from(value: PSInvocationState) -> Self {
        match value {
            PSInvocationState::NotStarted => Self::NotStarted,
            PSInvocationState::Running => Self::Running,
            PSInvocationState::Stopping => Self::Stopping,
            PSInvocationState::Stopped => Self::Stopped,
            PSInvocationState::Completed => Self::Completed,
            PSInvocationState::Failed => Self::Failed,
            PSInvocationState::Disconnected => Self::Disconnected,
        }
    }
}

/// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/b05495bc-a9b2-4794-9f43-4bf1f3633900
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RunspacePoolState {
    BeforeOpen = 0,
    Opening = 1,
    Opened = 2,
    Closed = 3,
    Closing = 4,
    Broken = 5,
    NegotiationSent = 6,
    NegotiationSucceeded = 7,
    Connecting = 8,
    Disconnected = 9,
}

impl From<&RunspacePoolStateValue> for RunspacePoolState {
    fn from(value: &RunspacePoolStateValue) -> Self {
        match value {
            RunspacePoolStateValue::BeforeOpen => Self::BeforeOpen,
            RunspacePoolStateValue::Opening => Self::Opening,
            RunspacePoolStateValue::Opened => Self::Opened,
            RunspacePoolStateValue::Closed => Self::Closed,
            RunspacePoolStateValue::Closing => Self::Closing,
            RunspacePoolStateValue::Broken => Self::Broken,
            RunspacePoolStateValue::NegotiationSent => Self::NegotiationSent,
            RunspacePoolStateValue::NegotiationSucceeded => Self::NegotiationSucceeded,
            RunspacePoolStateValue::Connecting => Self::Connecting,
            RunspacePoolStateValue::Disconnected => Self::Disconnected,
        }
    }
}
