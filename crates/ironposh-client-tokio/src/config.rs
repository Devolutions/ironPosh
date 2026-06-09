use anyhow::Context;
use clap::{Parser, ValueEnum};
use ironposh_client_core::{
    connector::{
        config::{KerberosConfig, TlsOptions},
        http::ServerAddress,
        WinRmConfig,
    },
    credentials::{ClientAuthIdentity, ClientUserName},
    AuthenticatorConfig, SspiAuthConfig, TransportSecurity,
};
use ironposh_psrp::{
    host_default_data::{HostDefaultData, Size},
    HostInfo,
};
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};
use url::Url;

/// PowerShell Remoting Client (Async/Tokio)
#[derive(Parser)]
#[command(version, about, long_about = None)]
// CLI flags are naturally independent booleans; grouping them adds no clarity here.
#[allow(clippy::struct_excessive_bools)]
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

    /// DANGEROUS: Accept any HTTPS server certificate (self-signed labs).
    /// Only meaningful together with `--https`.
    #[arg(
        long,
        help = "DANGEROUS: accept invalid HTTPS certificates (requires --https)"
    )]
    pub insecure: bool,

    /// Path to an additional root CA certificate (PEM) to trust for HTTPS.
    #[arg(long, help = "Path to an extra root CA certificate (PEM) for HTTPS")]
    pub ca_cert: Option<PathBuf>,

    /// Use parallel (multi-connection) session loop instead of the default serial mode.
    #[arg(
        long,
        help = "Use parallel session loop (default: serial/single-connection)"
    )]
    pub parallel: bool,

    /// Gateway base URL used to mimic the web demo path (for example http://localhost:7272).
    #[arg(long, help = "Use Gateway /jet/fwd/tcp WebSocket transport")]
    pub gateway: Option<String>,

    /// Gateway webapp username for /jet/webapp/app-token.
    #[arg(long, help = "Gateway webapp username (defaults to env or admin)")]
    pub gateway_webapp_username: Option<String>,

    /// Gateway webapp password for /jet/webapp/app-token.
    #[arg(long, help = "Gateway webapp password (defaults to env or admin)")]
    pub gateway_webapp_password: Option<String>,

    /// KDC address for Gateway KdcProxy token generation, e.g. tcp://dc.example.com:88.
    #[arg(long, help = "KDC address for Gateway KdcProxy token generation")]
    pub kdc_address: Option<String>,

    /// Full KDC proxy URL. When supplied, token generation for KDC is skipped.
    #[arg(long, help = "Full KDC proxy URL to use for Kerberos/Negotiate")]
    pub kdc_proxy_url: Option<String>,

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
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

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

/// Create connector configuration from command line arguments.
///
/// When `parallel` is false (default serial mode), `operation_timeout_secs` is set
/// to a short slice so inbound Receives don't block outbound sends for too long.
pub fn create_connector_config(args: &Args, cols: u16, rows: u16) -> anyhow::Result<WinRmConfig> {
    create_connector_config_with_kdc_url(args, cols, rows, None)
}

pub fn create_connector_config_with_kdc_url(
    args: &Args,
    cols: u16,
    rows: u16,
    kdc_url_override: Option<Url>,
) -> anyhow::Result<WinRmConfig> {
    let server = ServerAddress::parse(&args.server)?;

    // TLS flags only apply to HTTPS WinRM; reject meaningless combinations early.
    if args.insecure && !args.https {
        anyhow::bail!("--insecure only applies to HTTPS connections; add --https or drop --insecure");
    }

    let extra_ca_pem = args
        .ca_cert
        .as_ref()
        .map(|path| {
            std::fs::read(path)
                .with_context(|| format!("failed to read CA certificate file {}", path.display()))
        })
        .transpose()?;

    if args.insecure {
        tracing::warn!("Accepting invalid HTTPS certificates - this is INSECURE!");
    }

    let tls = TlsOptions {
        accept_invalid_certs: args.insecure,
        accept_invalid_hostnames: false,
        extra_ca_pem,
    };

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
                    kdc_url: kdc_url_override,
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
                    kdc_url: kdc_url_override,
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

    let host_info = HostInfo::builder()
        .host_default_data(host_data)
        .use_runspace_host(true)
        .build();

    // Serial mode uses a short timeout so Receives don't block outbound sends.
    // Keep Receive long-poll slices short to reduce perceived latency (initial connection
    // + Ctrl+C responsiveness) under a single in-flight HTTP constraint.
    let operation_timeout_secs = if args.parallel { None } else { Some(0.25) };

    Ok(WinRmConfig {
        server: (server, args.port),
        transport,
        authentication: auth,
        host_info,
        operation_timeout_secs,
        tls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironposh_client_core::connector::TransportSecurity;

    #[test]
    fn serial_mode_defaults_to_250ms_operation_timeout() {
        let args = Args {
            server: "127.0.0.1".to_string(),
            port: 5985,
            username: "user".to_string(),
            password: "pass".to_string(),
            domain: String::new(),
            auth_method: AuthMethod::Basic,
            https: false,
            http_insecure: true,
            insecure: false,
            ca_cert: None,
            parallel: false,
            gateway: None,
            gateway_webapp_username: None,
            gateway_webapp_password: None,
            kdc_address: None,
            kdc_proxy_url: None,
            verbose: 0,
            command: None,
        };

        let cfg = create_connector_config(&args, 120, 30).expect("create config");
        assert_eq!(cfg.transport, TransportSecurity::HttpInsecure);
        assert_eq!(cfg.operation_timeout_secs, Some(0.25));
    }

    #[test]
    fn parallel_mode_keeps_default_operation_timeout() {
        let args = Args {
            server: "127.0.0.1".to_string(),
            port: 5985,
            username: "user".to_string(),
            password: "pass".to_string(),
            domain: String::new(),
            auth_method: AuthMethod::Basic,
            https: false,
            http_insecure: true,
            insecure: false,
            ca_cert: None,
            parallel: true,
            gateway: None,
            gateway_webapp_username: None,
            gateway_webapp_password: None,
            kdc_address: None,
            kdc_proxy_url: None,
            verbose: 0,
            command: None,
        };

        let cfg = create_connector_config(&args, 120, 30).expect("create config");
        assert_eq!(cfg.operation_timeout_secs, None);
    }

    fn https_args() -> Args {
        Args {
            server: "127.0.0.1".to_string(),
            port: 5986,
            username: "user".to_string(),
            password: "pass".to_string(),
            domain: String::new(),
            auth_method: AuthMethod::Basic,
            https: true,
            http_insecure: false,
            insecure: false,
            ca_cert: None,
            parallel: false,
            gateway: None,
            gateway_webapp_username: None,
            gateway_webapp_password: None,
            kdc_address: None,
            kdc_proxy_url: None,
            verbose: 0,
            command: None,
        }
    }

    #[test]
    fn insecure_with_https_maps_to_tls_options() {
        let mut args = https_args();
        args.insecure = true;

        let cfg = create_connector_config(&args, 120, 30).expect("create config");
        assert!(cfg.tls.accept_invalid_certs);
        assert!(!cfg.tls.accept_invalid_hostnames);

        // The mapped options must be usable to construct the reqwest client.
        crate::http_client::build_reqwest_client(&cfg.tls).expect("client from mapped options");
    }

    #[test]
    fn ca_cert_flag_reads_pem_file() {
        let rcgen::CertifiedKey { cert, .. } =
            rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
        let pem = cert.pem();

        let path = std::env::temp_dir().join(format!("ironposh-test-ca-{}.pem", uuid::Uuid::new_v4()));
        std::fs::write(&path, &pem).expect("write temp CA pem");

        let mut args = https_args();
        args.ca_cert = Some(path.clone());

        let cfg = create_connector_config(&args, 120, 30).expect("create config");
        assert_eq!(cfg.tls.extra_ca_pem.as_deref(), Some(pem.as_bytes()));

        std::fs::remove_file(&path).expect("remove temp CA pem");
    }

    #[test]
    fn insecure_without_https_fails() {
        let mut args = https_args();
        args.https = false;
        args.insecure = true;

        let err = create_connector_config(&args, 120, 30).expect_err("must reject --insecure without --https");
        assert!(err.to_string().contains("--https"), "error should mention --https: {err}");
    }
}
