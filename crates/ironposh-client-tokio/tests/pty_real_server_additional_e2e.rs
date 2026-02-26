mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_burst_sequential_pipelines_no_sleep() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    for i in 1..=10 {
        h.send_line(&format!("Write-Output '__BURST_{i}__'"));
    }

    assert!(
        h.wait_for_output_contains("__BURST_10__", Duration::from_secs(25)),
        "last burst marker not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    for i in 1..=10 {
        let marker = format!("__BURST_{i}__");
        assert!(
            h.wait_for_output_contains(&marker, Duration::from_secs(1)),
            "missing burst marker: {marker}. tail={}",
            h.tail_string(16 * 1024)
        );
    }

    println!("TEST_SUCCESS: pty_real_server_additional_e2e::burst");
    h.send_line("exit");
}

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_large_output_last_line_arrives() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Large output to stress Receive draining + fragmentation.
    h.send_line("1..10000 | ForEach-Object { $_ }; Write-Output '__LARGE_DONE__'");

    assert!(
        h.wait_for_output_contains("__LARGE_DONE__", Duration::from_secs(120)),
        "large output marker not observed. tail={}",
        h.tail_string(32 * 1024)
    );
    assert!(
        h.wait_for_output_contains("10000", Duration::from_secs(2)),
        "expected last line (10000) not observed. tail={}",
        h.tail_string(32 * 1024)
    );

    println!("TEST_SUCCESS: pty_real_server_additional_e2e::large_output");
    h.send_line("exit");
}

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_terminating_error_does_not_break_next_pipeline() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // A terminating error.
    h.send_line("throw '__FATAL_TEST__'");

    // Next pipeline should still execute.
    h.send_line("Write-Output '__AFTER_THROW__'");
    assert!(
        h.wait_for_output_contains("__AFTER_THROW__", Duration::from_secs(25)),
        "marker not observed after terminating error. tail={}",
        h.tail_string(16 * 1024)
    );

    println!("TEST_SUCCESS: pty_real_server_additional_e2e::terminating_error");
    h.send_line("exit");
}

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_error_and_warning_streams_are_delivered() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    h.send_line(
        "Write-Output '__MIX_OUT__'; Write-Warning '__MIX_WARN__'; Write-Error '__MIX_ERR__'; Write-Output '__MIX_DONE__'",
    );

    assert!(
        h.wait_for_output_contains("__MIX_DONE__", Duration::from_secs(25)),
        "mix marker not observed. tail={}",
        h.tail_string(32 * 1024)
    );
    assert!(
        h.wait_for_output_contains("__MIX_OUT__", Duration::from_secs(1)),
        "stdout token missing. tail={}",
        h.tail_string(32 * 1024)
    );
    assert!(
        h.wait_for_output_contains("__MIX_WARN__", Duration::from_secs(1)),
        "warning token missing. tail={}",
        h.tail_string(32 * 1024)
    );
    assert!(
        h.wait_for_output_contains("__MIX_ERR__", Duration::from_secs(1)),
        "error token missing. tail={}",
        h.tail_string(32 * 1024)
    );

    println!("TEST_SUCCESS: pty_real_server_additional_e2e::streams");
    h.send_line("exit");
}

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_delayed_output_burst_recovers_after_silence() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Silence, then a burst. This exercises receive scheduler backoff and recovery.
    h.send_line(
        "Start-Sleep -Seconds 3; 1..500 | ForEach-Object { $_ }; Write-Output '__SILENCE_DONE__'",
    );
    assert!(
        h.wait_for_output_contains("__SILENCE_DONE__", Duration::from_secs(60)),
        "marker not observed after silence+burst. tail={}",
        h.tail_string(32 * 1024)
    );

    println!("TEST_SUCCESS: pty_real_server_additional_e2e::silence_burst");
    h.send_line("exit");
}
