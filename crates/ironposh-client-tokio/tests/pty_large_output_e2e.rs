mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_large_output_not_truncated() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Emit 10 000 lines followed by a sentinel marker.
    h.send_line("1..10000 | ForEach-Object { $_ }; Write-Output '__LARGE_DONE__'");

    assert!(
        h.wait_for_output_contains("__LARGE_DONE__", Duration::from_secs(60)),
        "large output done marker (__LARGE_DONE__) not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
