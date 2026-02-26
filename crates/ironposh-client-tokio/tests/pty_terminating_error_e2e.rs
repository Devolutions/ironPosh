mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_recovers_from_terminating_error() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Trigger a terminating error.
    h.send_line("throw \"fatal_test_error\"");
    std::thread::sleep(Duration::from_secs(3));

    // Prove the session is still alive after the throw.
    h.send_line("Write-Output '__AFTER_THROW__'");

    assert!(
        h.wait_for_output_contains("__AFTER_THROW__", Duration::from_secs(25)),
        "post-throw marker (__AFTER_THROW__) not observed; session may have died. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
