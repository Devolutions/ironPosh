//! Transport × Seal × AuthMethod matrix, asserted against a real WinRM server.
//!
//! Encodes the WinRM/WSMan rule (MS-WSMV; see Ansible/Microsoft docs):
//!
//! | Auth      | HTTP (no TLS)                    | HTTPS (TLS)                     |
//! |-----------|----------------------------------|---------------------------------|
//! | Basic     | refused (no encryption) — unless | allowed, never SSPI-sealed      |
//! |           | forced with `--http-insecure`    | (TLS provides confidentiality)  |
//! | NTLM      | allowed, SSPI message-sealed     | allowed, NOT sealed (TLS does)  |
//! | Kerberos  | allowed, SSPI message-sealed     | allowed, NOT sealed (TLS does)  |
//! | Negotiate | allowed, SSPI message-sealed     | allowed, NOT sealed (TLS does)  |
//!
//! Key invariant: **SSPI message sealing and TLS are mutually exclusive** — over
//! HTTPS the auth protocol must NOT seal, because TLS already encrypts.
//!
//! Notes on the test target (a hardened domain controller):
//! - Basic auth is disabled on the listener, so the Basic cells assert the
//!   *client-side* policy (refuse vs. permit/force) rather than a successful
//!   logon — that policy is what the matrix governs and is server-independent.
//! - The `*_over_https_does_not_seal` tests assert the spec-correct behaviour; if
//!   the client still seals over HTTPS they fail by design (that is the bug they
//!   guard against).

use ironposh_test_support::e2e_pwsh_config;
use std::process::Command;

struct ClientRun {
    success: bool,
    output: String,
    log: String,
}

/// Spawn the non-interactive client for one (auth, transport) cell and capture
/// its exit status, console output, and trace log.
fn run_cell(auth: &str, transport: &[&str], tag: &str) -> ClientRun {
    let bin = env!("CARGO_BIN_EXE_ironposh-client-tokio");
    let cfg = e2e_pwsh_config::load_from_env_or_default();
    let log_path =
        std::env::temp_dir().join(format!("ironposh-matrix.{tag}.{}.log", std::process::id()));
    let _ = std::fs::remove_file(&log_path);

    let mut cmd = Command::new(bin);
    cmd.env("IRONPOSH_TOKIO_LOG_FILE", &log_path);
    cmd.arg("--server").arg(&cfg.hostname);
    cmd.arg("--username").arg(&cfg.username);
    cmd.arg("--password").arg(&cfg.password);
    cmd.arg("--auth-method").arg(auth);
    for a in transport {
        cmd.arg(a);
    }
    cmd.arg("-c").arg("whoami");

    let out = cmd.output().expect("spawn non-interactive client");
    let log = std::fs::read_to_string(&log_path).unwrap_or_default();
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    ClientRun {
        success: out.status.success(),
        output,
        log,
    }
}

const HTTP: &[&str] = &["--port", "5985"];
const HTTPS: &[&str] = &["--port", "5986", "--https", "--insecure"];
const HTTP_FORCED: &[&str] = &["--port", "5985", "--http-insecure"];

/// The pipeline completed end-to-end (auth succeeded and a command ran).
fn connected(r: &ClientRun) -> bool {
    r.log.contains("pipeline finished")
}
/// The WinRM payload was SSPI message-sealed (`multipart/encrypted` envelope).
fn sealed(r: &ClientRun) -> bool {
    r.log.contains("multipart/encrypted")
}
/// The client made it past local policy onto the wire (got an HTTP response).
fn reached_server(r: &ClientRun) -> bool {
    r.log.contains("Received HTTP response") || connected(r)
}
fn tail(r: &ClientRun) -> String {
    let n = r.log.len().saturating_sub(1500);
    format!("output={}\n…log_tail={}", r.output.trim(), &r.log[n..])
}

// ─────────────────────────── HTTP + SSPI → sealed ───────────────────────────

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn negotiate_over_http_seals_messages() {
    let r = run_cell("negotiate", HTTP, "neg-http");
    assert!(
        connected(&r),
        "Negotiate/HTTP should authenticate and run\n{}",
        tail(&r)
    );
    assert!(
        sealed(&r),
        "Negotiate/HTTP MUST SSPI-seal the payload\n{}",
        tail(&r)
    );
}

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn kerberos_over_http_seals_messages() {
    let r = run_cell("kerberos", HTTP, "krb-http");
    assert!(
        connected(&r),
        "Kerberos/HTTP should authenticate and run\n{}",
        tail(&r)
    );
    assert!(
        sealed(&r),
        "Kerberos/HTTP MUST SSPI-seal the payload\n{}",
        tail(&r)
    );
}

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn ntlm_over_http_seals_messages() {
    let r = run_cell("ntlm", HTTP, "ntlm-http");
    assert!(
        connected(&r),
        "NTLM/HTTP should authenticate and run\n{}",
        tail(&r)
    );
    assert!(
        sealed(&r),
        "NTLM/HTTP MUST SSPI-seal the payload\n{}",
        tail(&r)
    );
}

// ──────────────────── HTTPS + SSPI → NOT sealed (TLS) ────────────────────

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn negotiate_over_https_does_not_seal() {
    let r = run_cell("negotiate", HTTPS, "neg-https");
    assert!(
        connected(&r),
        "Negotiate/HTTPS should authenticate and run\n{}",
        tail(&r)
    );
    assert!(
        !sealed(&r),
        "Negotiate/HTTPS MUST NOT SSPI-seal — TLS provides confidentiality (seal ⟂ TLS)\n{}",
        tail(&r)
    );
}

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn kerberos_over_https_does_not_seal() {
    let r = run_cell("kerberos", HTTPS, "krb-https");
    assert!(
        connected(&r),
        "Kerberos/HTTPS should authenticate and run\n{}",
        tail(&r)
    );
    assert!(
        !sealed(&r),
        "Kerberos/HTTPS MUST NOT SSPI-seal — TLS provides confidentiality (seal ⟂ TLS)\n{}",
        tail(&r)
    );
}

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn ntlm_over_https_does_not_seal() {
    let r = run_cell("ntlm", HTTPS, "ntlm-https");
    assert!(
        connected(&r),
        "NTLM/HTTPS should authenticate and run\n{}",
        tail(&r)
    );
    assert!(
        !sealed(&r),
        "NTLM/HTTPS MUST NOT SSPI-seal — TLS provides confidentiality (seal ⟂ TLS)\n{}",
        tail(&r)
    );
}

// ───────────────── Basic: disallowed over plain HTTP, force-gated ─────────────────

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn basic_over_http_is_refused_without_force() {
    let r = run_cell("basic", HTTP, "basic-http");
    assert!(
        !r.success,
        "Basic over plain HTTP must be refused\n{}",
        tail(&r)
    );
    assert!(
        r.output
            .contains("Basic authentication over plain HTTP is refused"),
        "refusal must be explained to the user\n{}",
        tail(&r)
    );
}

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn basic_over_http_is_allowed_with_force_flag() {
    let r = run_cell("basic", HTTP_FORCED, "basic-http-forced");
    assert!(
        !r.output
            .contains("Basic authentication over plain HTTP is refused"),
        "--http-insecure must bypass the Basic-over-HTTP guard\n{}",
        tail(&r)
    );
    assert!(
        reached_server(&r),
        "with --http-insecure the client should attempt the connection, not refuse locally\n{}",
        tail(&r)
    );
}

// ─────────────────── Basic over HTTPS: allowed, never sealed ───────────────────

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn basic_over_https_is_allowed_and_unsealed() {
    let r = run_cell("basic", HTTPS, "basic-https");
    assert!(
        !r.output
            .contains("Basic authentication over plain HTTP is refused"),
        "Basic over HTTPS must be permitted (TLS encrypts the credentials)\n{}",
        tail(&r)
    );
    assert!(
        !sealed(&r),
        "Basic is never SSPI-sealed; over HTTPS confidentiality comes from TLS\n{}",
        tail(&r)
    );
    assert!(
        reached_server(&r),
        "Basic/HTTPS should reach the server\n{}",
        tail(&r)
    );
}

// ─────────────── Forced unencrypted SSPI over HTTP → not sealed ───────────────

#[test]
#[ignore = "e2e matrix test: requires reachable WinRM server (HTTP 5985 + HTTPS 5986)"]
fn negotiate_over_forced_insecure_http_does_not_seal() {
    let r = run_cell("negotiate", HTTP_FORCED, "neg-http-insecure");
    // `--http-insecure` forces an unencrypted channel: the payload is sent plain
    // even for an SSPI auth method (the server may reject it; the client behaviour
    // under test is that it does NOT seal).
    assert!(
        !sealed(&r),
        "--http-insecure must send an unsealed (plain) payload\n{}",
        tail(&r)
    );
}
