use std::net::IpAddr;

use clap::Parser;
use pwsh_core::connector::{Authentication, ConnectorConfig, Scheme, http::ServerAddress};
use tracing_subscriber::EnvFilter;

/// PowerShell Remoting Client (Synchronous)
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Server IP address to connect to
    #[arg(short, long, default_value = "10.10.0.3", help = "Server IP address")]
    pub server: IpAddr,

    /// Server port to connect to
    #[arg(short, long, default_value = "5985", help = "Server port")]
    pub port: u16,

    /// Username for authentication
    #[arg(short, long, default_value = "Administrator", help = "Username for authentication")]
    pub username: String,

    /// Password for authentication
    #[arg(short = 'P', long, default_value = "DevoLabs123!", help = "Password for authentication")]
    pub password: String,

    /// Use HTTPS instead of HTTP
    #[arg(long, help = "Use HTTPS (default: HTTP)")]
    pub https: bool,

    /// Verbose logging (can be repeated for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase logging verbosity")]
    pub verbose: u8,
}

/// Initialize logging with file output
pub fn init_logging(verbose_level: u8) -> anyhow::Result<()> {
    let log_file = std::fs::File::create("winrm_client.log")?;
    
    // Determine log level based on verbosity
    let log_level = match verbose_level {
        0 => "powershell_sync=info,pwsh_core=info,protocol_powershell_remoting=info,protocol_winrm=warn,ureq=error",
        1 => "powershell_sync=debug,pwsh_core=debug,protocol_powershell_remoting=debug,protocol_winrm=info,ureq=error",
        2 => "powershell_sync=trace,pwsh_core=trace,protocol_powershell_remoting=trace,protocol_winrm=debug,ureq=warn",
        _ => "trace",
    };

    let max_level = match verbose_level {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(EnvFilter::new(log_level))
        .with_max_level(max_level)
        .with_target(false)
        .with_line_number(true)
        .with_file(true)
        .with_writer(log_file)
        .init();
    Ok(())
}

/// Create connector configuration from command line arguments
pub fn create_connector_config(args: &Args) -> ConnectorConfig {
    let server = ServerAddress::Ip(args.server);
    let scheme = if args.https { Scheme::Https } else { Scheme::Http };
    let auth = Authentication::Basic {
        username: args.username.clone(),
        password: args.password.clone(),
    };

    ConnectorConfig {
        server: (server, args.port),
        scheme,
        authentication: auth,
        host_info: protocol_powershell_remoting::HostInfo::builder()
            .is_host_null(false)
            .is_host_ui_null(true)
            .is_host_raw_ui_null(true)
            .build(),
    }
}