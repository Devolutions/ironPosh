use ironposh_test_support::e2e_pwsh_config;

use std::process::Command;

/// Run the client against the default session configuration by passing
/// `--configuration-name Microsoft.PowerShell` explicitly. This proves the
/// shell resource URI plumbing end-to-end against any WinRM server, without
/// requiring a custom JEA endpoint.
#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn explicit_default_configuration_name_executes_command() {
    let bin = env!("CARGO_BIN_EXE_ironposh-client-tokio");
    let mut cmd = Command::new(bin);

    let cfg = e2e_pwsh_config::load_from_env_or_default();

    cmd.arg("--server").arg(cfg.hostname);
    cmd.arg("--port").arg(cfg.port);
    cmd.arg("--username").arg(cfg.username);
    cmd.arg("--password").arg(cfg.password);
    if let Some(domain) = cfg.domain {
        cmd.arg("--domain").arg(domain);
    }
    cmd.arg("--auth-method")
        .arg(e2e_pwsh_config::default_auth_method());
    cmd.arg("--configuration-name").arg("Microsoft.PowerShell");
    cmd.arg("-c").arg("whoami");

    // Keep logs out of the repo root and make runs easier to inspect.
    let log_file = std::env::temp_dir().join("ironposh-client-tokio.configuration-name-e2e.log");
    cmd.env("IRONPOSH_TOKIO_LOG_FILE", log_file.as_os_str());

    let out = cmd.output().expect("spawn non-interactive tokio client");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "whoami via explicit configuration name failed. stdout={stdout} stderr={stderr}"
    );
    assert!(
        !stdout.trim().is_empty(),
        "whoami produced no output. stderr={stderr}"
    );
}
