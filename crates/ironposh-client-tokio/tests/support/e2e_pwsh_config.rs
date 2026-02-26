#[derive(Debug, Clone)]
pub struct Pwshe2eConfig {
    pub hostname: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
}

fn first_nonempty_env(names: &[&str]) -> Option<String> {
    for &n in names {
        if let Ok(v) = std::env::var(n) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

#[allow(dead_code)]
pub fn load_from_env() -> Option<Pwshe2eConfig> {
    // Mirror WebTerminal's E2E config env vars so running both suites uses the same
    // environment.
    //
    // Supported:
    // - IRONPOSH_E2E_* (tokio test-specific)
    // - E2E_PWSH_* (Playwright real-server e2e)
    // - VITE_PWSH_* (demo .env)
    let hostname = first_nonempty_env(&[
        "IRONPOSH_E2E_SERVER",
        "E2E_PWSH_HOSTNAME",
        "VITE_PWSH_HOSTNAME",
    ])?;
    let port = first_nonempty_env(&["IRONPOSH_E2E_PORT", "E2E_PWSH_PORT", "VITE_PWSH_PORT"])
        .unwrap_or_else(|| "5985".to_string());
    let username = first_nonempty_env(&[
        "IRONPOSH_E2E_USERNAME",
        "E2E_PWSH_USERNAME",
        "VITE_PWSH_USERNAME",
    ])?;
    let password = first_nonempty_env(&[
        "IRONPOSH_E2E_PASSWORD",
        "E2E_PWSH_PASSWORD",
        "VITE_PWSH_PASSWORD",
    ])?;
    let domain =
        first_nonempty_env(&["IRONPOSH_E2E_DOMAIN", "E2E_PWSH_DOMAIN", "VITE_PWSH_DOMAIN"]);

    Some(Pwshe2eConfig {
        hostname,
        port,
        username,
        password,
        domain,
    })
}

/// Load E2E config from environment if present, otherwise fall back to the
/// tokio client's CLI defaults.
///
/// This intentionally makes `--ignored` real-server tests runnable out of the
/// box in dev environments where the default target is reachable.
pub fn load_from_env_or_default() -> Pwshe2eConfig {
    // Keep these in sync with `crates/ironposh-client-tokio/src/config.rs` CLI defaults.
    const DEFAULT_HOSTNAME: &str = "IT-HELP-DC.ad.it-help.ninja";
    const DEFAULT_PORT: &str = "5985";
    const DEFAULT_USERNAME: &str = "Administrator@ad.it-help.ninja";
    const DEFAULT_PASSWORD: &str = "DevoLabs123!";

    let hostname = first_nonempty_env(&[
        "IRONPOSH_E2E_SERVER",
        "E2E_PWSH_HOSTNAME",
        "VITE_PWSH_HOSTNAME",
    ])
    .unwrap_or_else(|| DEFAULT_HOSTNAME.to_string());

    let port = first_nonempty_env(&["IRONPOSH_E2E_PORT", "E2E_PWSH_PORT", "VITE_PWSH_PORT"])
        .unwrap_or_else(|| DEFAULT_PORT.to_string());

    let username = first_nonempty_env(&[
        "IRONPOSH_E2E_USERNAME",
        "E2E_PWSH_USERNAME",
        "VITE_PWSH_USERNAME",
    ])
    .unwrap_or_else(|| DEFAULT_USERNAME.to_string());

    let password = first_nonempty_env(&[
        "IRONPOSH_E2E_PASSWORD",
        "E2E_PWSH_PASSWORD",
        "VITE_PWSH_PASSWORD",
    ])
    .unwrap_or_else(|| DEFAULT_PASSWORD.to_string());

    let domain =
        first_nonempty_env(&["IRONPOSH_E2E_DOMAIN", "E2E_PWSH_DOMAIN", "VITE_PWSH_DOMAIN"]);

    Pwshe2eConfig {
        hostname,
        port,
        username,
        password,
        domain,
    }
}
