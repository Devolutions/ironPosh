mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_mixed_streams_single_pipeline() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Exercise multiple PowerShell streams in a single pipeline.
    h.send_line(
        "Write-Output 'OUT'; Write-Warning 'WARN'; Write-Error 'ERR'; \
         Write-Verbose 'VERB' -Verbose; Write-Output '__MIX_DONE__'",
    );

    assert!(
        h.wait_for_output_contains("__MIX_DONE__", Duration::from_secs(25)),
        "mixed streams done marker (__MIX_DONE__) not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
