mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_rapid_sequential_pipelines() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Send 10 commands back-to-back with no sleep between them.
    for n in 0..10 {
        h.send_line(&format!("Write-Output '__SEQ_MARKER_{n}__'"));
    }

    // Wait for the last marker to arrive.
    assert!(
        h.wait_for_output_contains("__SEQ_MARKER_9__", Duration::from_secs(30)),
        "last sequential marker (__SEQ_MARKER_9__) not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    // Verify all 10 markers arrived.
    for n in 0..10 {
        let marker = format!("__SEQ_MARKER_{n}__");
        let count = h.count_output_occurrences(&marker);
        assert!(
            count >= 1,
            "marker {marker} not found (count={count}). tail={}",
            h.tail_string(16 * 1024)
        );
    }

    h.send_line("exit");
}
