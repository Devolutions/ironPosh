use pwsh_core::connector::active_session::UserEvent;

#[derive(Debug)]
pub struct NotifyMap {
    map: std::collections::HashMap<uuid::Uuid, UserEvent>,
}

impl NotifyMap {
    pub fn new() -> Self {
        Self {
            map: std::collections::HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: uuid::Uuid, event: UserEvent) {
        self.map.insert(id, event);
    }

    pub fn remove(&mut self, id: &uuid::Uuid) -> Option<UserEvent> {
        self.map.remove(id)
    }

    pub async fn receive(&mut self, id: &uuid::Uuid) -> Option<UserEvent> {
        todo!()
    }
}
