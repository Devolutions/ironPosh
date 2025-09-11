// pub struct AuthContext

/*
    Ok, let's think about the API, and how people will use it.


    let winrm_session = WinRMSession::new(...);

    match winrm_session.step()? {
        WinRMSessionStep::newConnectRequested(connector) => {
            loop {
                match connector.step()? {
                    ConnectorStepResult::SendBack(http_request) => {
                        let response = client.send_request(http_request, KeepAlive::NotNecessary)?;
                        connector.receive_response(response)?;
                    }
                    ConnectorStepResult::SendBackError(e) => {
                        anyhow::bail!("Connection failed: {}", e);
                    }
                    ConnectorStepResult::Done(active_session) => {
                        break Ok(active_session);
                    }
                }

            }
        }
    }

*/

use std::collections::HashMap;

use crate::connector::auth_sequence::AuthContext;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId {
    id: u32,
}

impl ConnectionId {
    // Private New, disallow external creation
    fn new(id: u32) -> Self {
        Self { id }
    }

    pub fn inner(&self) -> u32 {
        self.id
    }
}

#[derive(Debug)]
pub struct ConnectionContext {
    state: ConnectionState,
    auth: AuthContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    PreAuth,
    Idle,
    Pending,
    Closed,
}

#[derive(Debug, Default)]
pub struct ConnectionPool {
    connections: HashMap<ConnectionId, ConnectionState>,
}

impl ConnectionPool {
    pub fn get_idle_or_new_connection(&mut self) -> ConnectionId {
        let available_conn_id = self.connections.iter().find_map(|(conn_id, state)| {
            if matches!(state, ConnectionState::Idle) {
                Some(conn_id.id)
            } else {
                None
            }
        });

        if let Some(id) = available_conn_id {
            let conn_id = ConnectionId::new(id);
            self.connections
                .get_mut(&conn_id)
                .map(|s| *s = ConnectionState::Pending);
            return conn_id;
        }

        let new_id = ConnectionId {
            id: self.connections.len() as u32 + 1,
        };
        let to_return = ConnectionId::new(new_id.id);
        self.connections.insert(new_id, ConnectionState::Pending);

        to_return
    }

    pub fn mark_connection_idle(&mut self, conn_id: &ConnectionId) {
        if let Some(state) = self.connections.get_mut(conn_id) {
            *state = ConnectionState::Idle;
        }
    }

    pub fn mark_connection_closed(&mut self, conn_id: &ConnectionId) {
        if let Some(state) = self.connections.get_mut(conn_id) {
            *state = ConnectionState::Closed;
        }
    }
}
