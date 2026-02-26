mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_record_streams_and_clear_host_keep_session_alive() {
    const DONE: &str = "__PTY_RECORDS_DONE__";
    const AFTER: &str = "__PTY_RECORDS_AFTER__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Stabilize preferences so record streams won't prompt.
    h.send_line(
        "$WarningPreference='Continue'; \
         $VerbosePreference='Continue'; \
         $DebugPreference='Continue'; \
         $InformationPreference='Continue'; \
         $ConfirmPreference='None'; \
         Write-Output '__PTY_PREFS_SET__'",
    );
    assert!(
        h.wait_for_output_contains("__PTY_PREFS_SET__", Duration::from_secs(20)),
        "did not observe preferences marker. tail={}",
        h.tail_string(16 * 1024)
    );

    // Exercise record streams; assert on a final marker so we only fail on real breakage.
    h.send_line(&format!(
        "Write-Warning '__PTY_REC_WARN__'; \
         Write-Verbose '__PTY_REC_VERBOSE__'; \
         Write-Debug '__PTY_REC_DEBUG__'; \
         Write-Information '__PTY_REC_INFO__' -InformationAction Continue; \
         1..5 | % {{ Write-Progress -Activity 'E2E' -Status $_ -PercentComplete ($_*20); Start-Sleep -Milliseconds 50 }}; \
         Clear-Host; \
         Write-Output '{DONE}'"
    ));

    assert!(
        h.wait_for_output_contains(DONE, Duration::from_secs(30)),
        "did not observe records done marker. tail={}",
        h.tail_string(16 * 1024)
    );

    // Prove the session remains usable.
    h.send_line(&format!("Write-Output '{AFTER}'"));
    assert!(
        h.wait_for_output_contains(AFTER, Duration::from_secs(20)),
        "session did not produce after marker. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
