use clap::{Parser, ValueEnum};
use ironposh_client_core::{
    connector::{config::KerberosConfig, http::ServerAddress, WinRmConfig},
    AuthenticatorConfig, SspiAuthConfig, TransportSecurity,
};
use ironposh_psrp::{HostDefaultData, Size};
use std::sync::OnceLock;
use tracing::debug;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// Sets up a panic hook to ensure logs are flushed before the program exits.
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        // Log the panic information
        tracing::error!("A panic occurred: {}", panic_info);
    }));
}

/// PowerShell Remoting Client (Synchronous)
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

    /// Optional KDC URL for Kerberos authentication
    #[arg(
        long,
        help = "KDC URL for Kerberos authentication (e.g., IT-HELP-DC.ad.it-help.ninja)"
    )]
    pub kdc_url: Option<String>,

    /// Optional client computer name for Kerberos authentication
    #[arg(long, help = "Client computer name for Kerberos authentication")]
    pub client_computer_name: Option<String>,

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
    // Set up the panic hook to flush logs on panic
    setup_panic_hook();

    // Bridge logs from the `log` crate to `tracing`
    LogTracer::init().ok();

    let file = std::fs::File::create("winrm_client.log")?;
    let (nb_writer, guard) = tracing_appender::non_blocking(file);

    // Store the guard in a static OnceLock to ensure it lives for the duration of the program.
    if LOG_GUARD.set(guard).is_err() {
        // This case should ideally not happen in a single-threaded setup,
        // but it's good practice to handle it.
        tracing::warn!("LOG_GUARD was already set. This may indicate a problem in initialization.");
    }

    // Determine log level based on verbosity using global filters
    let filter_str = match verbose_level {
        0 => "info,ureq=error,sspi=error",
        1 => "debug,ureq=warn,sspi=error",
        2 => "trace,ureq=info,sspi=error",
        _ => "trace",
    };

    let env_filter = EnvFilter::new(filter_str);

    let subscriber = Registry::default().with(env_filter).with(
        fmt::layer()
            .with_writer(nb_writer)
            .with_target(true)
            .with_line_number(true)
            .with_file(true)
            .with_ansi(false)
            .compact(),
    );

    tracing::subscriber::set_global_default(subscriber)?;

    // Immediately log something to confirm initialization.
    tracing::info!("Logging system initialized.");

    Ok(())
}

/// Create connector configuration from command line arguments
pub fn create_connector_config(
    args: &Args,
    cols: u16,
    rows: u16,
) -> Result<WinRmConfig, anyhow::Error> {
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
            let client_username =
                ironposh_client_core::credentials::ClientUserName::new(&args.username, domain)?;
            let identity = ironposh_client_core::credentials::ClientAuthIdentity::new(
                client_username,
                args.password.clone(),
            );
            AuthenticatorConfig::Sspi(SspiAuthConfig::NTLM {
                target: args.server.clone(),
                identity,
            })
        }
        AuthMethod::Kerberos => {
            let client_username =
                ironposh_client_core::credentials::ClientUserName::new(&args.username, domain)?;

            let identity = ironposh_client_core::credentials::ClientAuthIdentity::new(
                client_username,
                args.password.clone(),
            );

            let kdc_url = args.kdc_url.as_ref().map(|url| url.parse()).transpose()?;

            AuthenticatorConfig::Sspi(SspiAuthConfig::Kerberos {
                target: args.server.clone(),
                identity,
                kerberos_config: KerberosConfig {
                    kdc_url,
                    client_computer_name: args.client_computer_name.clone().unwrap_or_else(|| {
                        whoami::fallible::hostname().unwrap_or_else(|_| "localhost".to_string())
                    }),
                },
            })
        }
        AuthMethod::Negotiate => {
            let client_username =
                ironposh_client_core::credentials::ClientUserName::new(&args.username, domain)?;
            let identity = ironposh_client_core::credentials::ClientAuthIdentity::new(
                client_username,
                args.password.clone(),
            );
            AuthenticatorConfig::Sspi(SspiAuthConfig::Negotiate {
                target: args.server.clone(),
                identity,
                kerberos_config: Some(KerberosConfig {
                    kdc_url: args.kdc_url.as_ref().map(|url| url.parse()).transpose()?,
                    client_computer_name: args.client_computer_name.clone().unwrap_or_else(|| {
                        whoami::fallible::hostname().unwrap_or_else(|_| "localhost".to_string())
                    }),
                }),
            })
        }
    };

    // Use real terminal size from the terminal instance
    debug!(
        cols,
        rows, "Using real terminal size from terminal instance"
    );

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

    let host_info = ironposh_psrp::HostInfo::builder()
        .host_default_data(host_data)
        .build();

    Ok(WinRmConfig {
        server: (server, args.port),
        transport,
        authentication: auth,
        host_info,
    })
}
