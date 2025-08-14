use std::hash::Hash;

use crate::runspace_pool::PsInvocationState;

#[derive(Debug, Clone)]
pub struct PipelineRepresentation {
    pub id: uuid::Uuid,
    pub state: PsInvocationState,
}

impl PipelineRepresentation {
    pub fn new(id: uuid::Uuid) -> Self {
        PipelineRepresentation {
            id,
            state: PsInvocationState::NotStarted,
        }
    }

    pub fn id(&self) -> uuid::Uuid {
        self.id
    }
}

impl Hash for PipelineRepresentation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for PipelineRepresentation {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PipelineRepresentation {}

/// Purely a reference to the underlying PowerShell instance inside the runspace pool.
#[derive(Debug, Clone)]
pub struct PowerShell {
    pub(crate) id: uuid::Uuid,
}

impl PowerShell {
    pub(crate) fn new(id: uuid::Uuid) -> Self {
        PowerShell { id }
    }

    pub fn id(&self) -> uuid::Uuid {
        self.id
    }
}

pub struct Runspace {
    pub id: uuid::Uuid,
    pub state: super::enums::RunspacePoolState,
}
