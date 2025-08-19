use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum HostCallType {
    Pipeline { id: Uuid },
    RunspacePool,
}
