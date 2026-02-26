mod e2e_auths;
mod support;

use std::process::Command;
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

    // Keep logs out of the repo root and make runs easier to inspect.
    let log_file = std::env::temp_dir().join("ironposh-client-tokio.command-e2e.log");
    cmd.env("IRONPOSH_TOKIO_LOG_FILE", log_file.as_os_str());

    cmd
}

fn run_noninteractive(auth: &str, script: &str) -> (bool, String, String) {
    let mut cmd = cmd_base();
    cmd.arg("--auth-method").arg(auth);
    cmd.arg("-c").arg(script);

    let out = cmd.output().expect("spawn non-interactive tokio client");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn real_server_command_matrix_all_auths() {
    for auth in e2e_auths::auths_from_env_or_default() {
        // C1: marker output
        {
            let marker = format!("__E2E_CMD_{auth}_C1__");
            let (ok, stdout, stderr) =
                run_noninteractive(auth, &format!("Write-Output '{marker}'"));
            assert!(
                ok,
                "C1 failed for auth={auth}. stdout={stdout} stderr={stderr}"
            );
            assert!(
                stdout.contains(&marker),
                "C1 marker missing for auth={auth}. stdout={stdout} stderr={stderr}"
            );
        }

        // C2: output is non-empty
        {
            let (ok, stdout, stderr) =
                run_noninteractive(auth, "Get-ChildItem | Select-Object -First 3");
            assert!(
                ok,
                "C2 failed for auth={auth}. stdout={stdout} stderr={stderr}"
            );
            assert!(
                !stdout.trim().is_empty(),
                "C2 empty stdout for auth={auth}. stderr={stderr}"
            );
        }

        // C3: connection failure then recover.
        //
        // WebTerminal does this by mutating the password field; here we do it by overriding `--password`.
        {
            let marker = format!("__E2E_CMD_{auth}_C3_RETRY__");

            let mut bad = cmd_base();
            bad.arg("--auth-method").arg(auth);
            bad.arg("--password").arg("wrong-password");
            bad.arg("-c").arg("Write-Output '__E2E_BAD_PASS__'");
            let bad_out = bad.output().expect("spawn with wrong password");
            assert!(
                !bad_out.status.success(),
                "C3 expected failure for auth={auth} but succeeded. stdout={} stderr={}",
                String::from_utf8_lossy(&bad_out.stdout),
                String::from_utf8_lossy(&bad_out.stderr)
            );

            let (ok, stdout, stderr) =
                run_noninteractive(auth, &format!("Write-Output '{marker}'"));
            assert!(
                ok,
                "C3 retry failed for auth={auth}. stdout={stdout} stderr={stderr}"
            );
            assert!(
                stdout.contains(&marker),
                "C3 retry marker missing for auth={auth}. stdout={stdout} stderr={stderr}"
            );
        }
    }
    let _ = Instant::now(); // keep `Instant` import used without adding behavior
}
