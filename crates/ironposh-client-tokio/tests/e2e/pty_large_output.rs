use ironposh_test_support::pty_harness::PtyHarness;
use std::path::Path;
use std::time::Duration;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_large_output_not_truncated() {
    let mut h =
        PtyHarness::try_spawn_tokio_client(Path::new(env!("CARGO_BIN_EXE_ironposh-client-tokio")));
    h.sleep_for_connect();

    // Emit 10 000 lines followed by a sentinel marker.
    h.send_line("1..10000 | ForEach-Object { $_ }; Write-Output '__LARGE_DONE__'");

    assert!(
        h.wait_for_output_contains("__LARGE_DONE__", Duration::from_mins(1)),
        "large output done marker (__LARGE_DONE__) not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
