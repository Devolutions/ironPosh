mod support;

use std::process::Command;

use support::e2e_pwsh_config::Pwshe2eConfig;

fn base_command(cfg: &Pwshe2eConfig) -> Command {
    let bin = env!("CARGO_BIN_EXE_ironposh-client-tokio");
    let mut cmd = Command::new(bin);

    cmd.arg("--server").arg(&cfg.hostname);
    cmd.arg("--port").arg(&cfg.port);
    cmd.arg("--username").arg(&cfg.username);
    cmd.arg("--password").arg(&cfg.password);
    if let Some(domain) = cfg.domain.as_deref() {
        cmd.arg("--domain").arg(domain);
    }
    if cfg.https {
        cmd.arg("--https");
    }
    if cfg.insecure {
        cmd.arg("--insecure");
    }
    if let Some(ca_cert) = cfg.ca_cert.as_deref() {
        cmd.arg("--ca-cert").arg(ca_cert);
    }

    cmd
}

fn run_marker(mut cmd: Command, marker: &str) {
    let log_file = std::env::temp_dir().join(format!(
        "ironposh-client-tokio.real-server-feature-e2e.{marker}.{}.log",
        std::process::id()
    ));
    cmd.env("IRONPOSH_TOKIO_LOG_FILE", log_file.as_os_str());
    cmd.arg("-c").arg(format!("Write-Output '{marker}'"));

    let out = cmd.output().expect("spawn non-interactive tokio client");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "real-server command failed. stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains(marker),
        "real-server marker missing. marker={marker} stdout={stdout} stderr={stderr}"
    );
}

#[test]
#[ignore = "e2e test: requires real WinRM HTTPS server + valid credentials"]
fn real_server_https_executes_command() {
    let Some(cfg) = support::e2e_pwsh_config::load_from_env() else {
        eprintln!("skipping: set IRONPOSH_E2E_* or web demo env vars for real-server HTTPS E2E");
        return;
    };
    if !cfg.https {
        eprintln!(
            "skipping: set IRONPOSH_E2E_HTTPS=1 or VITE_PWSH_TER_TRANSPORT=Tls for HTTPS E2E"
        );
        return;
    }

    run_marker(base_command(&cfg), "__IRONPOSH_REAL_SERVER_HTTPS__");
}

#[test]
#[ignore = "e2e test: requires real Gateway + WinRM server + valid credentials"]
fn real_server_gateway_executes_command() {
    let Some(cfg) = support::e2e_pwsh_config::load_from_env() else {
        eprintln!("skipping: set IRONPOSH_E2E_* or web demo env vars for real-server Gateway E2E");
        return;
    };
    let Some(gateway) = cfg.gateway.as_deref() else {
        eprintln!(
            "skipping: set IRONPOSH_E2E_GATEWAY or VITE_PWSH_TER_GATEWAY_URL for Gateway E2E"
        );
        return;
    };

    let mut cmd = base_command(&cfg);
    cmd.arg("--gateway").arg(gateway);
    if let Some(username) = cfg.gateway_webapp_username.as_deref() {
        cmd.arg("--gateway-webapp-username").arg(username);
    }
    if let Some(password) = cfg.gateway_webapp_password.as_deref() {
        cmd.arg("--gateway-webapp-password").arg(password);
    }

    run_marker(cmd, "__IRONPOSH_REAL_SERVER_GATEWAY__");
}

#[test]
#[ignore = "e2e test: requires real custom PowerShell configuration/JEA endpoint"]
fn real_server_custom_configuration_name_executes_command() {
    let Some(cfg) = support::e2e_pwsh_config::load_from_env() else {
        eprintln!("skipping: set IRONPOSH_E2E_* or web demo env vars for real-server JEA E2E");
        return;
    };
    let Some(configuration_name) = cfg.configuration_name.as_deref() else {
        eprintln!(
            "skipping: set IRONPOSH_E2E_CONFIGURATION_NAME or VITE_PWSH_TER_CONFIGURATION_NAME for custom JEA E2E"
        );
        return;
    };

    let mut cmd = base_command(&cfg);
    cmd.arg("--configuration-name").arg(configuration_name);

    run_marker(cmd, "__IRONPOSH_REAL_SERVER_CUSTOM_CONFIGURATION__");
}

#[test]
fn e2e_config_accepts_terminal_app_env_aliases() {
    let vars = [
        ("VITE_PWSH_TER_SERVER", "server.example.com"),
        ("VITE_PWSH_TER_TRANSPORT", "Tls"),
        ("VITE_PWSH_TER_USERNAME", "user@example.com"),
        ("VITE_PWSH_TER_PASSWORD", "secret"),
        ("VITE_PWSH_TER_DOMAIN", "EXAMPLE"),
        ("VITE_PWSH_TER_GATEWAY_URL", "http://gateway.example.com"),
        ("VITE_PWSH_TER_GATEWAY_WEBAPP_USERNAME", "gateway-user"),
        ("VITE_PWSH_TER_GATEWAY_WEBAPP_PASSWORD", "gateway-secret"),
    ];

    let cfg = support::e2e_pwsh_config::load_from_test_vars(|name| {
        vars.iter()
            .find_map(|(key, value)| (*key == name).then(|| (*value).to_string()))
    })
    .expect("terminal-app aliases should provide required config");

    assert_eq!(cfg.hostname, "server.example.com");
    assert_eq!(cfg.port, "5986");
    assert_eq!(cfg.username, "user@example.com");
    assert_eq!(cfg.password, "secret");
    assert_eq!(cfg.domain.as_deref(), Some("EXAMPLE"));
    assert!(cfg.https);
    assert_eq!(cfg.gateway.as_deref(), Some("http://gateway.example.com"));
    assert_eq!(cfg.gateway_webapp_username.as_deref(), Some("gateway-user"));
    assert_eq!(
        cfg.gateway_webapp_password.as_deref(),
        Some("gateway-secret")
    );
}

#[test]
fn e2e_config_prefers_explicit_port_over_https_default() {
    let vars = [
        ("IRONPOSH_E2E_SERVER", "server.example.com"),
        ("IRONPOSH_E2E_HTTPS", "1"),
        ("IRONPOSH_E2E_PORT", "9443"),
        ("IRONPOSH_E2E_USERNAME", "user@example.com"),
        ("IRONPOSH_E2E_PASSWORD", "secret"),
    ];

    let cfg = support::e2e_pwsh_config::load_from_test_vars(|name| {
        vars.iter()
            .find_map(|(key, value)| (*key == name).then(|| (*value).to_string()))
    })
    .expect("IRONPOSH_E2E aliases should provide required config");

    assert!(cfg.https);
    assert_eq!(cfg.port, "9443");
}
