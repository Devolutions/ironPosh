mod e2e_auths;
mod support;

use std::process::Command;
use std::time::Duration;
use std::time::Instant;

fn cmd_base() -> Command {
    let bin = env!("CARGO_BIN_EXE_ironposh-client-tokio");
    let mut cmd = Command::new(bin);

    let cfg = support::e2e_pwsh_config::load_from_env_or_default();

    cmd.arg("--server").arg(cfg.hostname);
    cmd.arg("--port").arg(cfg.port);
    cmd.arg("--username").arg(cfg.username);
    cmd.arg("--password").arg(cfg.password);
    if let Some(domain) = cfg.domain {
        cmd.arg("--domain").arg(domain);
    }

    let log_file = std::env::temp_dir().join("ironposh-client-tokio.latency-e2e.log");
    cmd.env("IRONPOSH_TOKIO_LOG_FILE", log_file.as_os_str());

    cmd
}

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_noninteractive_latency_sanity() {
    let auths = e2e_auths::auths_from_env_or_default();
    let auth = auths.first().expect("at least one auth method");

    let mut cmd = cmd_base();
    cmd.arg("--auth-method").arg(auth);
    cmd.arg("-c").arg("Write-Output '__LATENCY_OK__'");

    let start = Instant::now();
    let out = cmd.output().expect("spawn non-interactive tokio client");
    let elapsed = start.elapsed();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        elapsed < Duration::from_secs(15),
        "non-interactive command took too long: {elapsed:?}. stdout={stdout} stderr={stderr}"
    );
    assert!(
        out.status.success(),
        "non-interactive command failed. stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("__LATENCY_OK__"),
        "latency marker missing from stdout. stdout={stdout} stderr={stderr}"
    );
}
