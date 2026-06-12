#[derive(Debug, Clone)]
pub struct Pwshe2eConfig {
    pub hostname: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
    pub https: bool,
    pub insecure: bool,
    pub ca_cert: Option<String>,
    pub gateway: Option<String>,
    pub gateway_webapp_username: Option<String>,
    pub gateway_webapp_password: Option<String>,
    pub configuration_name: Option<String>,
}

const SERVER_ENV: &[&str] = &[
    "IRONPOSH_E2E_SERVER",
    "E2E_PWSH_HOSTNAME",
    "VITE_PWSH_HOSTNAME",
    "VITE_PWSH_TER_SERVER",
];
const PORT_ENV: &[&str] = &[
    "IRONPOSH_E2E_PORT",
    "E2E_PWSH_PORT",
    "VITE_PWSH_PORT",
    "VITE_PWSH_TER_PORT",
];
const USERNAME_ENV: &[&str] = &[
    "IRONPOSH_E2E_USERNAME",
    "E2E_PWSH_USERNAME",
    "VITE_PWSH_USERNAME",
    "VITE_PWSH_TER_USERNAME",
];
const PASSWORD_ENV: &[&str] = &[
    "IRONPOSH_E2E_PASSWORD",
    "E2E_PWSH_PASSWORD",
    "VITE_PWSH_PASSWORD",
    "VITE_PWSH_TER_PASSWORD",
];
const DOMAIN_ENV: &[&str] = &[
    "IRONPOSH_E2E_DOMAIN",
    "E2E_PWSH_DOMAIN",
    "VITE_PWSH_DOMAIN",
    "VITE_PWSH_TER_DOMAIN",
];
const HTTPS_BOOL_ENV: &[&str] = &[
    "IRONPOSH_E2E_HTTPS",
    "E2E_PWSH_HTTPS",
    "VITE_PWSH_HTTPS",
    "VITE_PWSH_TER_USE_HTTPS",
];
const HTTPS_TRANSPORT_ENV: &[&str] = &["VITE_PWSH_TER_TRANSPORT"];
const INSECURE_ENV: &[&str] = &["IRONPOSH_E2E_INSECURE", "E2E_PWSH_INSECURE"];
const CA_CERT_ENV: &[&str] = &["IRONPOSH_E2E_CA_CERT", "E2E_PWSH_CA_CERT"];
const GATEWAY_ENV: &[&str] = &[
    "IRONPOSH_E2E_GATEWAY",
    "E2E_GATEWAY_URL",
    "VITE_PWSH_GATEWAY",
    "VITE_PWSH_TER_GATEWAY_URL",
];
const GATEWAY_USERNAME_ENV: &[&str] = &[
    "IRONPOSH_E2E_GATEWAY_WEBAPP_USERNAME",
    "E2E_GATEWAY_WEBAPP_USERNAME",
    "VITE_GATEWAY_WEBAPP_USERNAME",
    "VITE_PWSH_TER_GATEWAY_WEBAPP_USERNAME",
];
const GATEWAY_PASSWORD_ENV: &[&str] = &[
    "IRONPOSH_E2E_GATEWAY_WEBAPP_PASSWORD",
    "E2E_GATEWAY_WEBAPP_PASSWORD",
    "VITE_GATEWAY_WEBAPP_PASSWORD",
    "VITE_PWSH_TER_GATEWAY_WEBAPP_PASSWORD",
];
const CONFIGURATION_NAME_ENV: &[&str] = &[
    "IRONPOSH_E2E_CONFIGURATION_NAME",
    "E2E_PWSH_CONFIGURATION_NAME",
    "VITE_PWSH_CONFIGURATION_NAME",
    "VITE_PWSH_TER_CONFIGURATION_NAME",
];

struct Defaults {
    hostname: &'static str,
    port: &'static str,
    username: &'static str,
    password: &'static str,
}

fn first_nonempty(
    names: &[&str],
    get_env: &mut impl FnMut(&str) -> Option<String>,
) -> Option<String> {
    for &n in names {
        if let Some(v) = get_env(n) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn first_bool(names: &[&str], get_env: &mut impl FnMut(&str) -> Option<String>) -> Option<bool> {
    let value = first_nonempty(names, get_env)?;
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn https_enabled(get_env: &mut impl FnMut(&str) -> Option<String>) -> bool {
    if let Some(value) = first_bool(HTTPS_BOOL_ENV, get_env) {
        return value;
    }

    first_nonempty(HTTPS_TRANSPORT_ENV, get_env)
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "tls" | "https" => Some(true),
            "tcp" | "http" => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

fn load_from_lookup(
    mut get_env: impl FnMut(&str) -> Option<String>,
    defaults: Option<&Defaults>,
) -> Option<Pwshe2eConfig> {
    let https = https_enabled(&mut get_env);
    let hostname = first_nonempty(SERVER_ENV, &mut get_env)
        .or_else(|| defaults.map(|d| d.hostname.to_string()))?;
    let port = first_nonempty(PORT_ENV, &mut get_env).unwrap_or_else(|| {
        if https {
            "5986".to_string()
        } else {
            defaults.map_or("5985", |d| d.port).to_string()
        }
    });
    let username = first_nonempty(USERNAME_ENV, &mut get_env)
        .or_else(|| defaults.map(|d| d.username.to_string()))?;
    let password = first_nonempty(PASSWORD_ENV, &mut get_env)
        .or_else(|| defaults.map(|d| d.password.to_string()))?;
    let domain = first_nonempty(DOMAIN_ENV, &mut get_env);
    let insecure = first_bool(INSECURE_ENV, &mut get_env).unwrap_or(false);
    let ca_cert = first_nonempty(CA_CERT_ENV, &mut get_env);
    let gateway = first_nonempty(GATEWAY_ENV, &mut get_env);
    let gateway_webapp_username = first_nonempty(GATEWAY_USERNAME_ENV, &mut get_env);
    let gateway_webapp_password = first_nonempty(GATEWAY_PASSWORD_ENV, &mut get_env);
    let configuration_name = first_nonempty(CONFIGURATION_NAME_ENV, &mut get_env);

    Some(Pwshe2eConfig {
        hostname,
        port,
        username,
        password,
        domain,
        https,
        insecure,
        ca_cert,
        gateway,
        gateway_webapp_username,
        gateway_webapp_password,
        configuration_name,
    })
}

pub fn load_from_test_vars(get_env: impl FnMut(&str) -> Option<String>) -> Option<Pwshe2eConfig> {
    load_from_lookup(get_env, None)
}

pub fn load_from_env() -> Option<Pwshe2eConfig> {
    // Mirror WebTerminal's E2E config env vars so running both suites uses the same
    // environment.
    //
    // Supported:
    // - IRONPOSH_E2E_* (tokio test-specific)
    // - E2E_PWSH_* (Playwright real-server e2e)
    // - VITE_PWSH_* / VITE_PWSH_TER_* (repo web demos)
    load_from_lookup(|name| std::env::var(name).ok(), None)
}

/// Load E2E config from environment if present, otherwise fall back to the
/// tokio client's CLI defaults.
///
/// This intentionally makes `--ignored` real-server tests runnable out of the
/// box in dev environments where the default target is reachable.
pub fn load_from_env_or_default() -> Pwshe2eConfig {
    // Keep these in sync with `crates/ironposh-client-tokio/src/config.rs` CLI defaults.
    load_from_lookup(
        |name| std::env::var(name).ok(),
        Some(&Defaults {
            hostname: "IT-HELP-DC.ad.it-help.ninja",
            port: "5985",
            username: "Administrator@ad.it-help.ninja",
            password: "DevoLabs123!",
        }),
    )
    .expect("defaults provide required E2E config fields")
}
