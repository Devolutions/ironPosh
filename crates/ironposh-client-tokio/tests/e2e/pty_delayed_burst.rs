use ironposh_test_support::pty_harness::PtyHarness;
use std::path::Path;
use std::time::Duration;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_delayed_then_burst_output() {
    let mut h =
        PtyHarness::try_spawn_tokio_client(Path::new(env!("CARGO_BIN_EXE_ironposh-client-tokio")));
    h.sleep_for_connect();

    // Sleep for 3 seconds then burst 500 lines of output.
    h.send_line("Start-Sleep 3; 1..500 | ForEach-Object { $_ }; Write-Output '__BURST_DONE__'");

    assert!(
        h.wait_for_output_contains("__BURST_DONE__", Duration::from_secs(30)),
        "burst done marker (__BURST_DONE__) not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
