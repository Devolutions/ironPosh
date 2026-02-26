mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_idle_ctrl_c_burst_does_not_disconnect() {
    const MARKER: &str = "__PTY_IDLE_CTRL_C_AFTER__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // No running pipeline: spam Ctrl+C and ensure the session survives.
    h.send_ctrl_c_burst(100, Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(500));

    h.send_line(&format!("Write-Output '{MARKER}'"));
    assert!(
        h.wait_for_output_contains(MARKER, Duration::from_secs(25)),
        "marker not observed after idle ctrl+c burst. tail={}",
        h.tail_string(16 * 1024)
    );

    println!("TEST_SUCCESS: pty_idle_ctrl_c_e2e");
    h.send_line("exit");
}
