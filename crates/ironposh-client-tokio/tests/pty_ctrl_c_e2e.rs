mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_ctrl_c_spam_does_not_break_session() {
    const MARKER: &str = "__PTY_E2E_AFTER_PING_SPAM__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // 1) Start a long-running command with continuous output.
    h.send_line("ping 8.8.8.8 -n 1000");
    std::thread::sleep(Duration::from_secs(5));

    // 2) Intensively spam Ctrl+C for a short period.
    h.send_ctrl_c_burst(30, Duration::from_millis(20));
    std::thread::sleep(Duration::from_secs(1));

    // 3) Run a follow-up command to prove the session is still alive.
    h.send_line(&format!("whoami; Write-Output '{MARKER}'"));

    assert!(
        h.wait_for_output_contains(MARKER, Duration::from_secs(25)),
        "marker not observed in terminal output; session may have disconnected. \
        tail={}",
        h.tail_string(16 * 1024)
    );

    // Best-effort cleanup.
    //
    // Important: do not `wait()` or `join()` here. In PTY + interactive terminal setups,
    // Windows console teardown can occasionally block indefinitely and wedge the test
    // harness. This is a local/ignored e2e test; prefer "no hang" over perfect cleanup.
    h.send_line("exit");
}
