use std::net::Ipv4Addr;

use pwsh_core::connector::{Authentication, ConnectorConfig, Scheme, http::ServerAddress};
use tracing_subscriber::EnvFilter;

/// Initialize logging with file output
pub fn init_logging() -> anyhow::Result<()> {
    let log_file = std::fs::File::create("winrm_client.log")?;
    tracing_subscriber::fmt::SubscriberBuilder::default()
        // Hide HTTP-related logs by setting them to ERROR level
        // Focus on our PowerShell remoting logic
        .with_env_filter(EnvFilter::new(
            "pwsh_core=debug,protocol_powershell_remoting=debug,protocol_winrm=info,ureq=error",
        ))
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
        host_info: protocol_powershell_remoting::HostInfo::enabled_all(),
    }
}
