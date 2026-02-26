mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_clean_exit() {
    const MARKER: &str = "__CLEANUP_OK__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Verify basic connectivity before exit.
    h.send_line(&format!("Write-Output '{MARKER}'"));

    assert!(
        h.wait_for_output_contains(MARKER, Duration::from_secs(25)),
        "cleanup marker not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    // Send exit and wait a moment for teardown.
    //
    // Important: do not assert process exit. PTY teardown can hang on Windows,
    // as noted in existing tests.
    h.send_line("exit");
    std::thread::sleep(Duration::from_secs(3));
}
