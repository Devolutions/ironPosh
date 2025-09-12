use std::net::IpAddr;

use clap::Parser;
use ironposh_client_core::{
    connector::{http::ServerAddress, Scheme, WinRmConfig},
    Authentication, SspiAuthConfig,
};
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

/// PowerShell Remoting Client (Async/Tokio)
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
    #[arg(
        short,
        long,
        default_value = "Administrator",
        help = "Username for authentication"
    )]
    pub username: String,

    /// Password for authentication
    #[arg(
        short = 'P',
        long,
        default_value = "DevoLabs123!",
        help = "Password for authentication"
    )]
    pub password: String,

    #[arg(
        short = 'd',
        long,
        help = "Domain for authentication (if needed)",
        default_value = ""
    )]
    pub domain: Option<String>,

    /// Use HTTPS instead of HTTP
    #[arg(long, help = "Use HTTPS (default: HTTP)")]
    pub https: bool,

    /// Verbose logging (can be repeated for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase logging verbosity")]
    pub verbose: u8,

    /// Command to execute (if provided, runs in non-interactive mode)
    #[arg(short = 'c', long, help = "Command to execute")]
    pub command: Option<String>,
}

/// Initialize logging with file output and proper structured logging
pub fn init_logging(verbose_level: u8) -> anyhow::Result<()> {
    let log_file = std::fs::File::create("ironposh_client.log")?;

    // Determine log level based on verbosity using structured filters
    let log_level = match verbose_level {
        0 => "ironposh_client_tokio=info,powershell_async=info,pwsh_core=info,protocol_powershell_remoting=info,protocol_winrm=warn,reqwest=error",
        1 => "ironposh_client_tokio=debug,powershell_async=debug,pwsh_core=debug,protocol_powershell_remoting=debug,protocol_winrm=debug,reqwest=error",
        2 => "ironposh_client_tokio=trace,powershell_async=trace,pwsh_core=trace,protocol_powershell_remoting=trace,protocol_winrm=debug,reqwest=warn",
        _ => "trace",
    };

    let env_filter = EnvFilter::new(log_level);

    let subscriber = Registry::default().with(env_filter).with(
        fmt::layer()
            .with_writer(log_file)
            .with_target(true)
            .with_line_number(true)
            .with_file(true)
            .compact(),
    );

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

/// Create connector configuration from command line arguments
pub fn create_connector_config(args: &Args) -> WinRmConfig {
    let server = ServerAddress::Ip(args.server);
    let scheme = if args.https {
        Scheme::Https
    } else {
        Scheme::Http
    };
    // let auth = Authentication::Basic {
    //     username: args.username.clone(),
    //     password: args.password.clone(),
    // };

    let auth = Authentication::Sspi(SspiAuthConfig::Negotiate {
        target: format!("{}:{}", args.server, args.port),
        identity: ironposh_client_core::ClientAuthIdentity::new(
            ironposh_client_core::credentials::ClientUserName::new(&args.username, None)
                .expect("Invalid username"),
            args.password.clone().into(),
        ),
        kerberos_config: None,
    });

    WinRmConfig {
        server: (server, args.port),
        scheme,
        authentication: auth,
        host_info: ironposh_psrp::HostInfo::builder()
            .is_host_null(false)
            .is_host_ui_null(true)
            .is_host_raw_ui_null(true)
            .build(),
    }
}
