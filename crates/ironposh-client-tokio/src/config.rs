use clap::{Parser, ValueEnum};
use ironposh_client_core::{
    connector::{http::ServerAddress, Scheme, WinRmConfig},
    credentials::{ClientAuthIdentity, ClientUserName},
    AuthenticatorConfig, KerberosConfig, SspiAuthConfig,
};
use ironposh_psrp::{
    host_default_data::{HostDefaultData, Size},
    HostInfo,
};
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

/// PowerShell Remoting Client (Async/Tokio)
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Server address to connect to
    #[arg(short, long, default_value = "10.10.0.3", help = "Server address")]
    pub server: String,

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
    pub domain: String,

    /// Authentication method
    #[arg(
        short,
        long,
        default_value = "negotiate",
        help = "Authentication method"
    )]
    pub auth_method: AuthMethod,

    /// Use HTTPS instead of HTTP
    #[arg(long, help = "Use HTTPS (default: HTTP)")]
    pub https: bool,

    #[arg(long, help = "No sspi encrypted session", default_value_t = false)]
    pub no_encryption: bool,

    /// Verbose logging (can be repeated for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase logging verbosity")]
    pub verbose: u8,

    /// Command to execute (if provided, runs in non-interactive mode)
    #[arg(short = 'c', long, help = "Command to execute")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AuthMethod {
    Basic,
    Ntlm,
    Kerberos,
    Negotiate,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::Basic => write!(f, "basic"),
            AuthMethod::Ntlm => write!(f, "ntlm"),
            AuthMethod::Kerberos => write!(f, "kerberos"),
            AuthMethod::Negotiate => write!(f, "negotiate"),
        }
    }
}

/// Initialize logging with file output and proper structured logging
pub fn init_logging(verbose_level: u8) -> anyhow::Result<()> {
    let log_file = std::fs::File::create("ironposh_client.log")?;

    // Determine log level based on verbosity using structured filters
    let log_level = match verbose_level {
        0 => "info,ureq=error,sspi=error",
        1 => "debug,ureq=warn,sspi=error",
        2 => "trace,ureq=info,sspi=error",
        _ => "trace",
    };

    let env_filter = EnvFilter::new(log_level);

    let subscriber = Registry::default().with(env_filter).with(
        fmt::layer()
            .with_writer(log_file)
            .with_target(true)
            .with_line_number(true)
            .with_file(true)
            .with_ansi(false)
            .compact(),
    );

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

/// Create connector configuration from command line arguments
pub fn create_connector_config(args: &Args, cols: u16, rows: u16) -> anyhow::Result<WinRmConfig> {
    let server = ServerAddress::parse(&args.server)?;
    let scheme = if args.https {
        Scheme::Https
    } else {
        Scheme::Http
    };

    let domain = if args.domain.trim().is_empty() {
        None
    } else {
        Some(args.domain.as_str())
    };

    let client_username = ClientUserName::new(&args.username, domain)?;
    let identity = ClientAuthIdentity::new(client_username, args.password.clone());

    let auth = AuthenticatorConfig::Sspi {
        sspi: SspiAuthConfig::Negotiate {
            target: args.server.clone(),
            identity,
            kerberos_config: Some(KerberosConfig {
                kdc_url: None,
                client_computer_name: whoami::fallible::hostname()
                    .unwrap_or_else(|_| "localhost".to_string()),
            }),
        },
        require_encryption: !args.no_encryption,
    };

    Ok(WinRmConfig {
        server: (server, args.port),
        scheme,
        authentication: auth,
        host_info: {
            let size = Size {
                width: cols as i32,
                height: rows as i32,
            };

            let host_data = HostDefaultData::builder()
                .buffer_size(size.clone())
                .window_size(size.clone())
                .max_window_size(size.clone())
                .max_physical_window_size(size)
                .build();

            HostInfo::builder().host_default_data(host_data).build()
        },
    })
}
