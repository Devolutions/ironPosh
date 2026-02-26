mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_readhost_ctrl_c_race() {
    const MARKER: &str = "__READHOST_SURVIVED__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // 1) Start Read-Host which blocks waiting for user input.
    h.send_line("$x = Read-Host 'Enter value'");
    std::thread::sleep(Duration::from_secs(3));

    // 2) Cancel the Read-Host with Ctrl+C burst.
    h.send_ctrl_c_burst(5, Duration::from_millis(50));
    std::thread::sleep(Duration::from_secs(2));

    // 3) Verify the session is still alive after cancelling Read-Host.
    h.send_line(&format!("Write-Output '{MARKER}'"));

    assert!(
        h.wait_for_output_contains(MARKER, Duration::from_secs(25)),
        "marker not observed after Read-Host ctrl+c race. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
