mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_second_cmd_while_first_runs() {
    const MARKER: &str = "__SECOND__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // 1) Start a command that takes a while to complete.
    h.send_line("Start-Sleep 5; Write-Output '__FIRST__'");
    std::thread::sleep(Duration::from_secs(1));

    // 2) Send a second command while the first is still running.
    h.send_line(&format!("Write-Output '{MARKER}'"));

    // The second marker may arrive after __FIRST__, or the first may be interrupted.
    assert!(
        h.wait_for_output_contains(MARKER, Duration::from_secs(30)),
        "second-command marker not observed while first was running. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
