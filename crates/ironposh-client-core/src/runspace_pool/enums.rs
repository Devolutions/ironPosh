use ironposh_psrp::RunspacePoolStateValue;

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

impl From<ironposh_psrp::PSInvocationState> for PsInvocationState {
    fn from(value: ironposh_psrp::PSInvocationState) -> Self {
        match value {
            ironposh_psrp::PSInvocationState::NotStarted => PsInvocationState::NotStarted,
            ironposh_psrp::PSInvocationState::Running => PsInvocationState::Running,
            ironposh_psrp::PSInvocationState::Stopping => PsInvocationState::Stopping,
            ironposh_psrp::PSInvocationState::Stopped => PsInvocationState::Stopped,
            ironposh_psrp::PSInvocationState::Completed => PsInvocationState::Completed,
            ironposh_psrp::PSInvocationState::Failed => PsInvocationState::Failed,
            ironposh_psrp::PSInvocationState::Disconnected => PsInvocationState::Disconnected,
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
            RunspacePoolStateValue::BeforeOpen => RunspacePoolState::BeforeOpen,
            RunspacePoolStateValue::Opening => RunspacePoolState::Opening,
            RunspacePoolStateValue::Opened => RunspacePoolState::Opened,
            RunspacePoolStateValue::Closed => RunspacePoolState::Closed,
            RunspacePoolStateValue::Closing => RunspacePoolState::Closing,
            RunspacePoolStateValue::Broken => RunspacePoolState::Broken,
            RunspacePoolStateValue::NegotiationSent => RunspacePoolState::NegotiationSent,
            RunspacePoolStateValue::NegotiationSucceeded => RunspacePoolState::NegotiationSucceeded,
            RunspacePoolStateValue::Connecting => RunspacePoolState::Connecting,
            RunspacePoolStateValue::Disconnected => RunspacePoolState::Disconnected,
        }
    }
}
