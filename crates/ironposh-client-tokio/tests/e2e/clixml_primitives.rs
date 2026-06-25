use ironposh_test_support::e2e_pwsh_config;
use std::process::Command;

/// Run a script non-interactively against the lab server and capture output.
fn run_script(script: &str) -> (bool, String, String) {
    let cfg = e2e_pwsh_config::load_from_env_or_default();
    let bin = env!("CARGO_BIN_EXE_ironposh-client-tokio");
    let mut cmd = Command::new(bin);

    cmd.arg("--server").arg(&cfg.hostname);
    cmd.arg("--port").arg(&cfg.port);
    cmd.arg("--username").arg(&cfg.username);
    cmd.arg("--password").arg(&cfg.password);
    // Negotiate over HTTP seals the channel (allowed); the default would be Basic,
    // which is refused on plain HTTP.
    cmd.arg("--auth-method").arg("negotiate");
    if let Some(domain) = cfg.domain.as_deref() {
        cmd.arg("--domain").arg(domain);
    }
    if cfg.https {
        cmd.arg("--https");
    }
    if cfg.insecure {
        cmd.arg("--insecure");
    }

    let log_file = std::env::temp_dir().join(format!(
        "ironposh-client-tokio.clixml-primitives-e2e.{}.log",
        std::process::id()
    ));
    cmd.env("IRONPOSH_TOKIO_LOG_FILE", log_file.as_os_str());
    cmd.arg("-c").arg(script);

    let out = cmd.output().expect("spawn non-interactive tokio client");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Emits natively-typed pipeline output so the server serializes each value as
/// its own CLIXML primitive (`<Db>`, `<Sg>`, `<D>`, `<I16>`, `<U16>`, `<By>`,
/// `<SB>`, `<URI>`) rather than a string. Before the primitive-coverage work the
/// client errored on these tags, so the values never reached stdout; this pins
/// that they now deserialize and render. Values are exactly representable and
/// distinctive so the substring assertions can't collide.
#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn clixml_primitive_types_deserialize_from_server() {
    let script = "[double]1234.5; [single]789.5; [decimal]27.1828; [int16]-32109; \
                  [uint16]64206; [byte]251; [sbyte]-123; \
                  [uri]'https://ironposh.test/clixml-marker'";

    let (ok, stdout, stderr) = run_script(script);
    assert!(ok, "command failed. stdout={stdout} stderr={stderr}");

    for needle in [
        "1234.5",                      // Double  <Db>
        "789.5",                       // Single  <Sg>
        "27.1828",                     // Decimal <D>
        "-32109",                      // Int16   <I16>
        "64206",                       // UInt16  <U16>
        "251",                         // Byte    <By>
        "-123",                        // SByte   <SB>
        "ironposh.test/clixml-marker", // Uri     <URI>
    ] {
        assert!(
            stdout.contains(needle),
            "expected {needle:?} in output (CLIXML primitive failed to deserialize?). \
             stdout={stdout} stderr={stderr}"
        );
    }
}
