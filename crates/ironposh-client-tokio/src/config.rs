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

    /// Path to an additional root CA certificate (single PEM certificate, not a bundle) to trust for HTTPS.
    #[arg(
        long,
        help = "Path to an extra root CA certificate for HTTPS (single PEM certificate; bundles not supported)"
    )]
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

    /// PowerShell session configuration (JEA endpoint) name.
    #[arg(
        long,
        help = "PowerShell session configuration name (JEA endpoint, default: Microsoft.PowerShell)"
    )]
    pub configuration_name: Option<String>,

    /// Command to execute (if provided, runs in non-interactive mode)
    #[arg(short = 'c', long, help = "Command to execute")]
    pub command: Option<String>,

    /// Reattach to an existing disconnected runspace pool shell by ShellId
    /// (printed by `:disconnect`). Requires the parallel session loop.
    #[arg(
        long,
        value_name = "UUID",
        help = "Reattach to a disconnected shell by ShellId (requires --parallel)"
    )]
    pub connect_shell_id: Option<uuid::Uuid>,
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

/// Validate `--gateway`-specific flag combinations.
///
/// Called early in `main`, before any network call to the gateway, so invalid input
/// fails fast instead of after token requests / connectivity attempts.
pub fn validate_gateway_flags(args: &Args) -> anyhow::Result<()> {
    let Some(gateway) = args.gateway.as_deref() else {
        return Ok(());
    };

    // The Gateway WebSocket transport serializes HTTP requests over a single socket, which
    // the parallel (multi-connection) session loop cannot use.
    if args.parallel {
        anyhow::bail!(
            "--gateway does not support --parallel because the Gateway WebSocket transport serializes HTTP requests; omit --parallel"
        );
    }

    // TLS to the target is terminated by the gateway, so client-side target TLS knobs
    // have no effect here.
    if args.insecure || args.ca_cert.is_some() {
        anyhow::bail!(
            "TLS to the target is terminated by the gateway; \
             --insecure/--ca-cert have no effect with --gateway"
        );
    }

    // The client-to-Gateway hop must be TLS. The Gateway token handshake sends the webapp
    // credentials with HTTP Basic auth over this hop *before* any WinRM/SSPI sealing, and
    // (unless SSPI-sealed) the WinRM payload rides the same hop — so a plaintext ws://gateway
    // leaks credentials and traffic regardless of the WinRM auth method. Require https/wss,
    // with one exception: a loopback gateway, where plaintext never leaves the local machine
    // (this keeps `--gateway http://localhost:7171` dev workflows working).
    let scheme = gateway
        .split_once("://")
        .map(|(scheme, _)| scheme.to_ascii_lowercase())
        .unwrap_or_default();
    let is_tls_scheme = scheme == "https" || scheme == "wss";
    if !is_tls_scheme && !gateway_url_is_loopback(gateway) {
        anyhow::bail!(
            "--gateway requires an https:// or wss:// Gateway URL: the Gateway token \
             handshake sends credentials over the client-to-Gateway hop. Plaintext is only \
             allowed for a loopback gateway (localhost/127.0.0.1/::1)."
        );
    }

    Ok(())
}

/// Whether the gateway URL points at a loopback host (where plaintext stays on the local
/// machine). Handles scheme-less inputs (e.g. `localhost:7171`) by prepending a scheme
/// before parsing, since `Url::parse` would otherwise misread the host as the scheme.
fn gateway_url_is_loopback(gateway: &str) -> bool {
    let with_scheme = if gateway.contains("://") {
        gateway.to_string()
    } else {
        format!("ws://{gateway}")
    };
    let Ok(url) = url::Url::parse(&with_scheme) else {
        return false;
    };
    match url.host() {
        Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
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

    // Note: `--gateway` + `--insecure`/`--ca-cert` is rejected earlier in `main`, before
    // any gateway network call.

    // TLS flags only apply to HTTPS WinRM; reject meaningless combinations early.
    if args.insecure && !args.https {
        anyhow::bail!(
            "--insecure only applies to HTTPS connections; add --https or drop --insecure"
        );
    }

    if args.ca_cert.is_some() && !args.https {
        anyhow::bail!("--ca-cert only applies to HTTPS connections; add --https or drop --ca-cert");
    }

    let extra_ca_pem = args
        .ca_cert
        .as_ref()
        .map(|path| {
            let pem = std::fs::read(path).with_context(|| {
                format!("failed to read CA certificate file {}", path.display())
            })?;
            // Validate eagerly so a bad PEM fails at startup instead of inside the HTTP client.
            reqwest::Certificate::from_pem(&pem).with_context(|| {
                format!(
                    "failed to parse CA certificate file {} as a PEM certificate",
                    path.display()
                )
            })?;
            Ok::<_, anyhow::Error>(pem)
        })
        .transpose()?;

    if args.insecure {
        tracing::warn!("Accepting invalid HTTPS certificates - this is INSECURE!");
        eprintln!("Accepting invalid HTTPS certificates - this is INSECURE!");
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

    // Basic (and Certificate) auth carry credentials with no message-level
    // encryption, so they are only safe over TLS. WinRM refuses them on a plain
    // HTTP listener unless `AllowUnencrypted` is set; mirror that here. Refuse
    // Basic over plain HTTP unless the user explicitly forces an unencrypted
    // channel with `--http-insecure`.
    if matches!(args.auth_method, AuthMethod::Basic) && !args.https && !args.http_insecure {
        anyhow::bail!(
            "Basic authentication over plain HTTP is refused: credentials would be sent \
             unencrypted. Use --https, or force an unencrypted channel with --http-insecure."
        );
    }

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
        configuration_name: args.configuration_name.clone(),
    })
}

pub fn build_reattach_command_prefix(args: &Args) -> String {
    let mut parts = vec![
        "--server".to_string(),
        quote_command_arg(&args.server),
        "--port".to_string(),
        args.port.to_string(),
        "--username".to_string(),
        quote_command_arg(&args.username),
        "--auth-method".to_string(),
        args.auth_method.to_string(),
    ];

    if !args.domain.trim().is_empty() {
        parts.push("--domain".to_string());
        parts.push(quote_command_arg(&args.domain));
    }
    if args.https {
        parts.push("--https".to_string());
    }
    if args.http_insecure {
        parts.push("--http-insecure".to_string());
    }
    if args.insecure {
        parts.push("--insecure".to_string());
    }
    if let Some(ca_cert) = &args.ca_cert {
        parts.push("--ca-cert".to_string());
        parts.push(quote_command_arg(&ca_cert.display().to_string()));
    }
    if let Some(gateway) = &args.gateway {
        parts.push("--gateway".to_string());
        parts.push(quote_command_arg(gateway));
    }
    if let Some(username) = &args.gateway_webapp_username {
        parts.push("--gateway-webapp-username".to_string());
        parts.push(quote_command_arg(username));
    }
    if let Some(kdc_address) = &args.kdc_address {
        parts.push("--kdc-address".to_string());
        parts.push(quote_command_arg(kdc_address));
    }
    if let Some(kdc_proxy_url) = &args.kdc_proxy_url {
        parts.push("--kdc-proxy-url".to_string());
        parts.push(quote_command_arg(kdc_proxy_url));
    }
    if let Some(configuration_name) = &args.configuration_name {
        parts.push("--configuration-name".to_string());
        parts.push(quote_command_arg(configuration_name));
    }

    parts.push("--parallel".to_string());
    parts.push("--connect-shell-id".to_string());
    parts.join(" ")
}

pub fn build_reattach_credentials_hint(args: &Args) -> String {
    let mut flags = vec!["--password"];
    if args.gateway.is_some() {
        flags.push("--gateway-webapp-password");
    }

    format!(
        "credentials are not included; add {} if needed",
        flags.join(" and ")
    )
}

fn quote_command_arg(arg: &str) -> String {
    if !arg.is_empty()
        && arg.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '-' | '_' | '.' | ':' | '/' | '\\' | '@' | '=' | '?' | '%'
                )
        })
    {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "''"))
    }
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
            configuration_name: None,
            command: None,
            connect_shell_id: None,
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
            configuration_name: None,
            command: None,
            connect_shell_id: None,
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
            configuration_name: None,
            command: None,
            connect_shell_id: None,
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

        let path =
            std::env::temp_dir().join(format!("ironposh-test-ca-{}.pem", uuid::Uuid::new_v4()));
        std::fs::write(&path, &pem).expect("write temp CA pem");

        let mut args = https_args();
        args.ca_cert = Some(path.clone());

        let cfg = create_connector_config(&args, 120, 30).expect("create config");
        assert_eq!(cfg.tls.extra_ca_pem.as_deref(), Some(pem.as_bytes()));

        std::fs::remove_file(&path).expect("remove temp CA pem");
    }

    #[test]
    fn configuration_name_flag_maps_to_config() {
        let args = Args::parse_from([
            "ironposh-client-tokio",
            "--http-insecure",
            "--configuration-name",
            "Foo",
        ]);

        let cfg = create_connector_config(&args, 120, 30).expect("create config");
        assert_eq!(cfg.configuration_name.as_deref(), Some("Foo"));
    }

    #[test]
    fn absent_configuration_name_flag_maps_to_none() {
        let args = Args::parse_from(["ironposh-client-tokio", "--http-insecure"]);

        let cfg = create_connector_config(&args, 120, 30).expect("create config");
        assert_eq!(cfg.configuration_name, None);
    }

    #[test]
    fn connect_shell_id_flag_parses_uuid() {
        let args = Args::parse_from([
            "ironposh-client-tokio",
            "--http-insecure",
            "--parallel",
            "--connect-shell-id",
            "2d6534d0-6b12-40e3-b773-cba26459cfa8",
        ]);

        assert_eq!(
            args.connect_shell_id,
            Some("2d6534d0-6b12-40e3-b773-cba26459cfa8".parse().unwrap())
        );
    }

    #[test]
    fn absent_connect_shell_id_flag_maps_to_none() {
        let args = Args::parse_from(["ironposh-client-tokio", "--http-insecure"]);
        assert_eq!(args.connect_shell_id, None);
    }

    #[test]
    fn connect_shell_id_flag_rejects_invalid_uuid() {
        let result =
            Args::try_parse_from(["ironposh-client-tokio", "--connect-shell-id", "not-a-uuid"]);
        assert!(result.is_err(), "invalid UUID must fail to parse");
    }

    #[test]
    fn reattach_command_prefix_carries_connection_options() {
        let args = Args::parse_from([
            "ironposh-client-tokio",
            "--server",
            "dc01.example.com",
            "--port",
            "5986",
            "--username",
            "Administrator@ad.it-help.ninja",
            "--domain",
            "AD",
            "--auth-method",
            "kerberos",
            "--https",
            "--insecure",
            "--gateway",
            "https://gateway.example.com",
            "--gateway-webapp-username",
            "gateway-admin",
            "--kdc-address",
            "tcp://dc01.example.com:88",
            "--configuration-name",
            "JEA Endpoint",
            "--parallel",
        ]);

        let prefix = build_reattach_command_prefix(&args);

        assert!(prefix.contains("--server dc01.example.com"));
        assert!(prefix.contains("--port 5986"));
        assert!(prefix.contains("--username Administrator@ad.it-help.ninja"));
        assert!(prefix.contains("--domain AD"));
        assert!(prefix.contains("--auth-method kerberos"));
        assert!(prefix.contains("--https"));
        assert!(prefix.contains("--insecure"));
        assert!(prefix.contains("--gateway https://gateway.example.com"));
        assert!(prefix.contains("--gateway-webapp-username gateway-admin"));
        assert!(prefix.contains("--kdc-address tcp://dc01.example.com:88"));
        assert!(prefix.contains("--configuration-name 'JEA Endpoint'"));
        assert!(prefix.ends_with("--parallel --connect-shell-id"));
        assert!(!prefix.contains(&args.password));

        let credentials_hint = build_reattach_credentials_hint(&args);
        assert!(credentials_hint.contains("--password"));
        assert!(credentials_hint.contains("--gateway-webapp-password"));
        assert!(!credentials_hint.contains(&args.password));
    }

    #[test]
    fn insecure_without_https_fails() {
        let mut args = https_args();
        args.https = false;
        args.insecure = true;

        let err = create_connector_config(&args, 120, 30)
            .expect_err("must reject --insecure without --https");
        assert!(
            err.to_string().contains("--https"),
            "error should mention --https: {err}"
        );
    }

    #[test]
    fn gateway_with_insecure_fails() {
        let mut args = https_args();
        args.gateway = Some("wss://localhost:7272".to_string());
        args.insecure = true;

        let err = validate_gateway_flags(&args).expect_err("must reject --insecure with --gateway");
        assert!(
            err.to_string().contains("--gateway"),
            "error should mention --gateway: {err}"
        );
    }

    #[test]
    fn gateway_with_ca_cert_fails() {
        let mut args = https_args();
        args.gateway = Some("wss://localhost:7272".to_string());
        args.ca_cert = Some(PathBuf::from("unused.pem"));

        let err = validate_gateway_flags(&args).expect_err("must reject --ca-cert with --gateway");
        assert!(
            err.to_string().contains("--gateway"),
            "error should mention --gateway: {err}"
        );
    }

    #[test]
    fn gateway_nonloopback_plaintext_rejected() {
        // A plaintext (http://) gateway to a non-loopback host leaks the webapp credentials
        // (Basic auth on the token handshake) and, unless SSPI-sealed, the WinRM payload.
        // Rejected regardless of auth method.
        for auth in [AuthMethod::Basic, AuthMethod::Negotiate] {
            let mut args = https_args();
            args.https = false;
            args.auth_method = auth;
            args.gateway = Some("http://gw.example.com:7272".to_string());

            let err = validate_gateway_flags(&args)
                .expect_err("non-loopback plaintext gateway must be rejected");
            assert!(
                err.to_string().contains("https://") || err.to_string().contains("wss://"),
                "error should require a TLS gateway URL: {err}"
            );
        }
    }

    #[test]
    fn gateway_nonloopback_scheme_less_rejected() {
        let mut args = https_args();
        args.https = false;
        args.gateway = Some("gw.example.com:7272".to_string());

        let err = validate_gateway_flags(&args)
            .expect_err("non-loopback scheme-less gateway must be rejected");
        assert!(
            err.to_string().contains("https://") || err.to_string().contains("wss://"),
            "error should require a TLS gateway URL: {err}"
        );
    }

    #[test]
    fn gateway_tls_url_accepted() {
        for url in ["wss://gw.example.com:7272", "https://gw.example.com:7272"] {
            let mut args = https_args();
            args.gateway = Some(url.to_string());
            validate_gateway_flags(&args)
                .unwrap_or_else(|e| panic!("TLS gateway URL {url} must be accepted: {e}"));
        }
    }

    #[test]
    fn gateway_loopback_plaintext_allowed() {
        // Plaintext to a loopback gateway never leaves the machine, so it's allowed (dev).
        for url in [
            "http://localhost:7171",
            "localhost:7171",
            "http://127.0.0.1:7171",
            "ws://[::1]:7171",
        ] {
            let mut args = https_args();
            args.https = false;
            args.auth_method = AuthMethod::Basic;
            args.gateway = Some(url.to_string());
            validate_gateway_flags(&args)
                .unwrap_or_else(|e| panic!("loopback gateway {url} must be allowed: {e}"));
        }
    }

    #[test]
    fn gateway_with_parallel_fails() {
        let mut args = https_args();
        args.https = false;
        args.gateway = Some("http://localhost:7272".to_string());
        args.parallel = true;

        let err = validate_gateway_flags(&args).expect_err("must reject --parallel with --gateway");
        assert!(
            err.to_string().contains("--parallel"),
            "error should mention --parallel: {err}"
        );
    }

    #[test]
    fn ca_cert_without_https_fails() {
        let mut args = https_args();
        args.https = false;
        args.ca_cert = Some(PathBuf::from("unused.pem"));

        let err = create_connector_config(&args, 120, 30)
            .expect_err("must reject --ca-cert without --https");
        assert!(
            err.to_string().contains("--https"),
            "error should mention --https: {err}"
        );
    }

    #[test]
    fn ca_cert_with_invalid_pem_fails() {
        let path =
            std::env::temp_dir().join(format!("ironposh-test-bad-ca-{}.pem", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"this is not a PEM certificate").expect("write temp garbage pem");

        let mut args = https_args();
        args.ca_cert = Some(path.clone());

        let err =
            create_connector_config(&args, 120, 30).expect_err("must reject invalid PEM content");
        let msg = format!("{err:#}");
        assert!(
            msg.contains(&path.display().to_string()),
            "error should mention the file path: {msg}"
        );

        std::fs::remove_file(&path).expect("remove temp garbage pem");
    }

    #[test]
    fn quote_command_arg_single_quotes_query_string_url() {
        let url = "https://gw/jet/KdcProxy/tok?x=1&y=2";
        let quoted = quote_command_arg(url);
        assert!(
            quoted.starts_with('\''),
            "URL with PowerShell metacharacters must be single-quoted: {quoted}"
        );
        assert!(
            quoted.contains(url),
            "quoted argument must contain the full URL: {quoted}"
        );
    }
}
