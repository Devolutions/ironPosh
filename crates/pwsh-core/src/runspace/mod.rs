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

// pub struct Pipeline

pub struct Runspace {
    pub id: uuid::Uuid,
    pub state: RunspacePoolState,
}

pub struct RunspacePool {
    id: uuid::Uuid,
    runspaces: Vec<Runspace>,
    state: RunspacePoolState,
}

impl RunspacePool {
    pub fn new() -> Self {
        RunspacePool {
            id: uuid::Uuid::new_v4(),
            runspaces: Vec::new(),
            state: RunspacePoolState::BeforeOpen,
        }
    }

    pub fn add_runspace(&mut self, runspace: Runspace) {
        self.runspaces.push(runspace);
    }

    pub fn get_runspaces(&self) -> &Vec<Runspace> {
        &self.runspaces
    }
}
