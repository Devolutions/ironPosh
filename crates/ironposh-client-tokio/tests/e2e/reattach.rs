//! Reattach e2e: disconnect a shell from one client process, then attach a
//! brand-new client process to it with `--connect-shell-id` and verify the
//! runspace state survived.
//!
//! Requires the parallel session loop (`--parallel`) on both processes.

use ironposh_test_support::pty_harness::PtyHarness;
use std::path::Path;
use std::time::Duration;

/// Extract `needle`-prefixed token from `haystack`, stopping at the first
/// character not in `allowed`.
fn extract_after<'a>(
    haystack: &'a str,
    needle: &str,
    allowed: fn(char) -> bool,
) -> Option<&'a str> {
    let start = haystack.rfind(needle)? + needle.len();
    let rest = &haystack[start..];
    let end = rest.find(|c| !allowed(c)).unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn parallel_tokio_client_reattaches_disconnected_shell_from_new_process() {
    // ---- Process A: create state, disconnect, exit. ----
    let mut a = PtyHarness::try_spawn_tokio_client_with_args(
        Path::new(env!("CARGO_BIN_EXE_ironposh-client-tokio")),
        &["--parallel"],
    );
    a.sleep_for_connect();

    a.send_line("$marker = \"alive-$(Get-Random)\"");
    assert!(
        a.wait_for_output_contains(">", Duration::from_secs(20)),
        "did not observe prompt after setting marker. tail={}",
        a.tail_string(16 * 1024)
    );

    // Read the marker value back so process B can assert on it.
    a.send_line("Write-Output \"MARKER=$marker\"");
    assert!(
        a.wait_for_output_contains("MARKER=alive-", Duration::from_secs(30)),
        "did not observe marker echo. tail={}",
        a.tail_string(16 * 1024)
    );
    let tail = a.tail_string(64 * 1024);
    let marker = extract_after(&tail, "MARKER=", |c| c.is_ascii_alphanumeric() || c == '-')
        .expect("parse marker value from output")
        .to_owned();
    assert!(
        marker.starts_with("alive-"),
        "unexpected marker value: {marker}"
    );

    // Disconnect; the REPL prints the shell id and a reattach hint.
    a.send_line(":disconnect");
    assert!(
        a.wait_for_output_contains(
            "Disconnected from runspace pool (ShellId:",
            Duration::from_secs(30)
        ),
        "did not observe disconnect confirmation. tail={}",
        a.tail_string(16 * 1024)
    );
    let tail = a.tail_string(64 * 1024);
    let shell_id = extract_after(&tail, "--connect-shell-id ", |c| {
        c.is_ascii_hexdigit() || c == '-'
    })
    .expect("parse shell id from reattach hint")
    .to_owned();
    assert_eq!(shell_id.len(), 36, "unexpected shell id: {shell_id}");

    // Exit while disconnected: the shell must survive on the server.
    a.send_line("exit");
    drop(a);

    // ---- Process B: fresh client reattaches and reads the marker. ----
    let b = PtyHarness::try_spawn_tokio_client_with_args(
        Path::new(env!("CARGO_BIN_EXE_ironposh-client-tokio")),
        &[
            "--parallel",
            "--connect-shell-id",
            &shell_id,
            "-c",
            "Write-Output \"REATTACHED=$marker\"",
        ],
    );

    assert!(
        b.wait_for_output_contains(&format!("REATTACHED={marker}"), Duration::from_mins(1)),
        "runspace state was not preserved across processes. tail={}",
        b.tail_string(16 * 1024)
    );
}
