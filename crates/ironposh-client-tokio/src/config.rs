use clap::{Parser, ValueEnum};
use ironposh_client_core::{
    connector::{config::KerberosConfig, http::ServerAddress, WinRmConfig},
    credentials::{ClientAuthIdentity, ClientUserName},
    AuthenticatorConfig, SspiAuthConfig, TransportSecurity,
};
use ironposh_psrp::{
    host_default_data::{HostDefaultData, Size},
    HostInfo,
};
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

/// PowerShell Remoting Client (Async/Tokio)
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Server IP address to connect to
    #[arg(
        short,
        long,
        default_value = "IT-HELP-DC.ad.it-help.ninja",
        help = "Server IP address or hostname"
    )]
    pub server: String,

    /// Server port to connect to
    #[arg(short, long, default_value = "5985", help = "Server port")]
    pub port: u16,

    /// Username for authentication
    #[arg(
        short,
        long,
        default_value = "Administrator@ad.it-help.ninja",
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
        short,
        long,
        help = "Optional domain for authentication",
        default_value = ""
    )]
    pub domain: String,

    #[arg(short, long, help = "Authentication method", default_value_t = AuthMethod::Basic)]
    pub auth_method: AuthMethod,

    /// Use HTTPS instead of HTTP (TLS provides security, SSPI sealing not needed)
    #[arg(long, help = "Use HTTPS (default: HTTP)")]
    pub https: bool,

    /// DANGEROUS: Use HTTP without SSPI message sealing.
    /// Only use for testing/debugging. Post-auth messages will be unencrypted!
    #[arg(
        long,
        help = "DANGEROUS: HTTP without SSPI sealing (insecure, for testing only)"
    )]
    pub http_insecure: bool,

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
            Self::Basic => write!(f, "basic"),
            Self::Ntlm => write!(f, "ntlm"),
            Self::Kerberos => write!(f, "kerberos"),
            Self::Negotiate => write!(f, "negotiate"),
        }
    }
}

/// Initialize logging with file output and proper structured logging
pub fn init_logging(verbose_level: u8) -> anyhow::Result<()> {
    const DEFAULT_LOG_FILE: &str = "ironposh-client-tokio.log";
    const LOG_FILE_ENV: &str = "IRONPOSH_TOKIO_LOG_FILE";

    let log_file_path = std::env::var_os(LOG_FILE_ENV)
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from(DEFAULT_LOG_FILE), PathBuf::from);

    if let Some(parent) = log_file_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Append to preserve previous runs (use env override to point elsewhere if needed).
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;

    // Determine log level based on verbosity using structured filters
    let log_level = match verbose_level {
        0 => "info,ureq=error,sspi=error",
        1 => "debug,ureq=warn,sspi=error",
        2 => "trace,ureq=info,sspi=error",
        _ => "trace",
    };

    // Allow overriding filters with `RUST_LOG`.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

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
    tracing::info!(
        log_file = %log_file_path.display(),
        log_file_env = LOG_FILE_ENV,
        "tracing initialized (file output)"
    );
    Ok(())
}

/// Create connector configuration from command line arguments
pub fn create_connector_config(args: &Args, cols: u16, rows: u16) -> anyhow::Result<WinRmConfig> {
    let server = ServerAddress::parse(&args.server)?;

    // Determine transport security from CLI flags
    let transport = if args.https {
        TransportSecurity::Https
    } else if args.http_insecure {
        tracing::warn!("Using HTTP without SSPI sealing - this is INSECURE!");
        TransportSecurity::HttpInsecure
    } else {
        TransportSecurity::Http
    };

    let domain = if args.domain.trim().is_empty() {
        None
    } else {
        Some(args.domain.as_str())
    };

    let auth = match args.auth_method {
        AuthMethod::Basic => AuthenticatorConfig::Basic {
            username: args.username.clone(),
            password: args.password.clone(),
        },
        AuthMethod::Ntlm => {
            let client_username = ClientUserName::new(&args.username, domain)?;
            let identity = ClientAuthIdentity::new(client_username, args.password.clone());
            AuthenticatorConfig::Sspi(SspiAuthConfig::NTLM {
                target: args.server.clone(),
                identity,
            })
        }
        AuthMethod::Kerberos => {
            let client_username = ClientUserName::new(&args.username, domain)?;
            let identity = ClientAuthIdentity::new(client_username, args.password.clone());
            AuthenticatorConfig::Sspi(SspiAuthConfig::Kerberos {
                target: args.server.clone(),
                identity,
                kerberos_config: KerberosConfig {
                    kdc_url: None,
                    client_computer_name: whoami::fallible::hostname()
                        .unwrap_or_else(|_| "localhost".to_string()),
                },
            })
        }
        AuthMethod::Negotiate => {
            let client_username = ClientUserName::new(&args.username, domain)?;
            let identity = ClientAuthIdentity::new(client_username, args.password.clone());
            AuthenticatorConfig::Sspi(SspiAuthConfig::Negotiate {
                target: args.server.clone(),
                identity,
                kerberos_config: Some(KerberosConfig {
                    kdc_url: None,
                    client_computer_name: whoami::fallible::hostname()
                        .unwrap_or_else(|_| "localhost".to_string()),
                }),
            })
        }
    };

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

    let host_info = HostInfo::builder().host_default_data(host_data).build();

    Ok(WinRmConfig {
        server: (server, args.port),
        transport,
        authentication: auth,
        host_info,
    })
}
