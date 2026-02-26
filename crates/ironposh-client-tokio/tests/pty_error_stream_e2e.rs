mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_error_stream_delivery() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Write a non-terminating error followed by a normal output marker.
    h.send_line("Write-Error \"boom\"; Write-Output '__ERROR_DONE__'");

    assert!(
        h.wait_for_output_contains("__ERROR_DONE__", Duration::from_secs(25)),
        "error stream done marker (__ERROR_DONE__) not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
