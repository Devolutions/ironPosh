mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_nested_script_blocks() {
    const MARKER: &str = "__NESTED_OK__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Run nested script blocks.
    h.send_line(&format!("& {{ & {{ Write-Output '{MARKER}' }} }}"));

    assert!(
        h.wait_for_output_contains(MARKER, Duration::from_secs(25)),
        "marker not observed from nested script blocks. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
