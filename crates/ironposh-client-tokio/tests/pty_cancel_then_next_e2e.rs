mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_cancel_then_immediate_next() {
    const MARKER: &str = "__CANCEL_NEXT__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // 1) Start a long-running command.
    h.send_line("ping 8.8.8.8 -n 100");
    std::thread::sleep(Duration::from_secs(5));

    // 2) Cancel with Ctrl+C burst.
    h.send_ctrl_c_burst(5, Duration::from_millis(50));

    // 3) Immediately send a follow-up command (no sleep after Ctrl+C).
    h.send_line(&format!("Write-Output '{MARKER}'"));

    assert!(
        h.wait_for_output_contains(MARKER, Duration::from_secs(25)),
        "marker not observed after cancel-then-next. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
