use std::collections::HashMap;

use crate::{
    Authentication,
    connector::{
        Scheme, WinRmConfig,
        auth_sequence::{AuthContext, AuthSequence},
        encryption::EncryptionProvider,
        http::{HttpRequest, ServerAddress},
    },
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId {
    id: u32,
}

impl ConnectionId {
    // Private New, disallow external creation
    fn new(id: u32) -> Self {
        Self { id }
    }

    fn private_clone(&self) -> Self {
        Self { id: self.id }
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

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionState {
    PreAuth,
    Idle {
        encryption_provder: EncryptionProvider,
    },
    Pending {
        encryption_provder: EncryptionProvider,
    },
    Closed,
}

#[derive(Debug)]
pub struct ConnectionPool {
    connections: HashMap<ConnectionId, ConnectionState>,
    http_builder: crate::connector::http::HttpBuilder,
}

#[derive(Debug)]
pub struct PostConAuthSequence {
    auth_sequence: AuthSequence,
    request_to_send: HttpRequest,
}

impl PostConAuthSequence {
    // Has everything the sam
    // step like all other auth sequences
    // when finish, send the request
    // then push the connection id back to the pool
}

pub struct ConnectionPoolConfig {
    pub server: (ServerAddress, u16),
    pub scheme: Scheme,
    pub authentication: Authentication,
    pub require_encryption: bool,
}

impl From<&WinRmConfig> for ConnectionPoolConfig {
    fn from(
        WinRmConfig {
            server,
            scheme,
            authentication,
            require_encryption,
            ..
        }: &WinRmConfig,
    ) -> Self {
        Self {
            server: server.clone(),
            scheme: scheme.clone(),
            authentication: authentication.clone(),
            require_encryption: *require_encryption,
        }
    }
}

#[derive(Debug)]
pub enum TrySend {
    JustSend {
        request: HttpRequest,
        conn_id: ConnectionId,
    },
    AuthNeeded {
        auth_sequence: PostConAuthSequence,
        conn_id: ConnectionId,
    },
}

impl ConnectionPool {
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let http_builder = crate::connector::http::HttpBuilder::new(
            config.server.0,
            config.server.1,
            config.scheme,
        );

        Self {
            connections: HashMap::new(),
            http_builder,
        }
    }

    pub fn send(
        &mut self,
        unecrypted_winrm_request: &str,
    ) -> Result<TrySend, crate::PwshCoreError> {
        let con = self.get_idel();
        if let Some((id, encryption)) = con {
            let maybe_encrypted_body = encryption.encrypt(unecrypted_winrm_request)?;
            let http = self.http_builder.post(maybe_encrypted_body);

            return Ok(TrySend::JustSend {
                request: http,
                conn_id: id,
            });
        }

        let new_con_id = self.get_new_con_id();

        let auth_sequence = 

        // construct a new auth sequence here

        todo!("Create new connection if no idle connections");
    }

    fn get_new_con_id(&mut self) -> ConnectionId {
        let new_id = self.connections.len() as u32 + 1;
        self.connections
            .insert(ConnectionId::new(new_id), ConnectionState::PreAuth);
        ConnectionId::new(new_id)
    }

    fn get_idel(&mut self) -> Option<(ConnectionId, &mut EncryptionProvider)> {
        self.connections.iter_mut().find_map(|(conn_id, state)| {
            let ConnectionState::Idle { encryption_provder } = state else {
                return None;
            };

            Some((conn_id.private_clone(), encryption_provder))
        })
    }
}
