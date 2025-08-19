use std::net::Ipv4Addr;

use pwsh_core::connector::{Authentication, ConnectorConfig, Scheme, http::ServerAddress};
use tracing_subscriber::EnvFilter;

/// Initialize logging with file output
pub fn init_logging() -> anyhow::Result<()> {
    let log_file = std::fs::File::create("winrm_client.log")?;
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(EnvFilter::new("pwsh_core=debug,ureq=warn"))
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_line_number(true)
        .with_file(true)
        .with_writer(log_file)
        .init();
    Ok(())
}

/// Create the default connector configuration
pub fn create_connector_config() -> ConnectorConfig {
    // Configuration - modify these for your test server
    let server = ServerAddress::Ip(std::net::IpAddr::V4(Ipv4Addr::new(10, 10, 0, 3))); // Change to your server
    let port = 5985;
    let scheme = Scheme::Http;
    let auth = Authentication::Basic {
        username: "Administrator".to_string(),
        password: "DevoLabs123!".to_string(),
    };

    ConnectorConfig {
        server: (server, port),
        scheme,
        authentication: auth,
    }
}