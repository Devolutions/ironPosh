mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_tab_completion_completes_command_names() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Type a partial command, press Tab, and assert the completed command appears in the PTY output.
    h.send_bytes(b"Get-Ser");
    h.send_bytes(b"\t");

    assert!(
        h.wait_for_output_contains("Get-Service", Duration::from_secs(20)),
        "tab completion did not render expected completion. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
