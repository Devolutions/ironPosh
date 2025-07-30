use std::{collections::HashMap, net::IpAddr};

use base64::Engine;
use hyper::header::{CONNECTION, CONTENT_TYPE, HOST, USER_AGENT};

use uuid::Uuid;
mod builders;

use crate::PwshCoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpSchema {
    Http,
    Https,
}

impl std::fmt::Display for HttpSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpSchema::Http => write!(f, "http"),
            HttpSchema::Https => write!(f, "https"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Host {
    Domain(String),
    IpAddress(IpAddr),
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Host::Domain(domain) => write!(f, "{}", domain),
            Host::IpAddress(ip) => write!(f, "{}", ip),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    pub scheme: HttpSchema,
    // Representing a domain or IP address
    pub host: Host,
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub enum Auth {
    Basic(Credentials),
    SSPI, // Not implemented yet
}

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ConnectorConfig {
    pub target: Target,
    #[builder(default = 5000)]
    pub timeout: u64,
    #[builder(default = None, setter(strip_option))]
    pub auth: Option<Auth>,
}

impl ConnectorConfig {
    pub fn get_wsman_uri(&self) -> String {
        let host = self.target.host.to_string();
        let scheme = self.target.scheme.to_string();
        let uri_str = format!("{}://{}:5985/wsman?PSVersion=7.4.10", scheme, host);
        uri_str
    }
}

pub enum ConnectorState {
    BeforeConnect,
    Connecting { runspace_pool_id: Uuid },
    Connected,
}

pub struct Connector {
    pub state: ConnectorState,
    pub config: ConnectorConfig,
}

pub struct ConnectorRequest {
    pub body: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ConnectorResult {
    pub message: String,
    pub headers: HashMap<String, String>,
}

impl ConnectorResult {
    pub fn new(message: String) -> Self {
        ConnectorResult {
            message,
            headers: HashMap::new(),
        }
    }
}

impl Connector {
    pub fn new(config: ConnectorConfig) -> Self {
        Connector {
            state: ConnectorState::BeforeConnect,
            config,
        }
    }

    pub fn step(
        &mut self,
        message: Option<ConnectorResult>,
    ) -> Result<ConnectorResult, PwshCoreError> {
        match self.state {
            ConnectorState::BeforeConnect => {
                let runspace_pool_id = Uuid::new_v4();
                self.state = ConnectorState::Connecting { runspace_pool_id };
                return self.init_runspace_pool(runspace_pool_id);
            }
            ConnectorState::Connecting { runspace_pool_id } => {
                self.state = ConnectorState::Connected;
                todo!("Handle connection establishment logic");
            }
            ConnectorState::Connected => {
                todo!("Handle further steps after connection established");
            }
        }
    }

    fn init_runspace_pool(
        &self,
        runspace_pool_id: uuid::Uuid,
    ) -> Result<ConnectorResult, PwshCoreError> {
        // Generate unique IDs for this session
        let shell_id = Uuid::new_v4().to_string().to_uppercase();
        let message_id = format!("uuid:{}", Uuid::new_v4().to_string().to_uppercase());
        let session_id = format!("uuid:{}", Uuid::new_v4().to_string().to_uppercase());
        let operation_id = format!("uuid:{}", Uuid::new_v4().to_string().to_uppercase());

        // Convert UUIDs to byte arrays for PowerShell remoting
        let pid = Uuid::new_v4();
        let object_id = 0;

        // Generate creation XML using PowerShell remoting protocol
        let creation_xml = builders::create_creation_xml(runspace_pool_id, pid, object_id)
            .map_err(|e| {
                PwshCoreError::ConnectorError(format!("Failed to create creation XML: {}", e))
            })?;

        // Build endpoint URL
        let endpoint_url = format!(
            "{}://{}:5985/wsman?PSVersion=7.4.10",
            self.config.target.scheme.to_string(),
            self.config.target.host.to_string()
        );

        let config = builders::WinRmInitializationConfig::builder()
            .endpoint_url(endpoint_url.as_str())
            .shell_id(shell_id.as_str())
            .message_id(message_id.as_str())
            .session_id(session_id.as_str())
            .operation_id(operation_id.as_str())
            .creation_xml(creation_xml.as_str())
            .build();

        let xml = builders::initialization_winrm_xml(config);

        let mut headers = HashMap::from([
            (CONNECTION.to_string(), "keep-alive".to_string()),
            (USER_AGENT.to_string(), "pwsh-core/0.1.0".to_string()),
            (
                HOST.to_string(),
                format!("{}:5985", self.config.target.host.to_string()),
            ),
            (
                CONTENT_TYPE.to_string(),
                "application/soap+xml; charset=utf-8".to_string(),
            ),
        ]);

        if let Some(auth) = &self.config.auth {
            if let Auth::Basic(creds) = auth {
                headers.insert(
                    "Authorization".to_string(),
                    format!(
                        "Basic {}",
                        base64::engine::general_purpose::STANDARD
                            .encode(format!("{}:{}", creds.username, creds.password))
                    ),
                );
            }
        }

        // For now, return the XML as the message
        // HTTP handling are not easy to implement
        Ok(ConnectorResult {
            message: xml,
            headers,
        })
    }
}
