use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostCallScope {
    Pipeline { command_id: Uuid },
    RunspacePool,
}
