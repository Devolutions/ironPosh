mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_ctrl_c_during_read_host_prompt_does_not_disconnect() {
    const PROMPT_MARKER: &str = "__PTY_PROMPT_CTRL_C__";
    const AFTER: &str = "__PTY_AFTER_PROMPT_CTRL_C__";

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    h.send_line(&format!(
        "$x = Read-Host '{PROMPT_MARKER}'; Write-Output '__PTY_PROMPT_VALUE__=' + $x"
    ));

    // Wait until the prompt text is visible.
    assert!(
        h.wait_for_output_contains(PROMPT_MARKER, Duration::from_secs(25)),
        "did not observe Read-Host prompt. tail={}",
        h.tail_string(16 * 1024)
    );

    // Hit Ctrl+C instead of answering the prompt.
    h.send_bytes(&[0x03]);
    std::thread::sleep(Duration::from_millis(500));

    // Session must remain usable.
    h.send_line(&format!("Write-Output '{AFTER}'"));
    assert!(
        h.wait_for_output_contains(AFTER, Duration::from_secs(25)),
        "marker not observed after ctrl+c during prompt. tail={}",
        h.tail_string(16 * 1024)
    );

    println!("TEST_SUCCESS: pty_prompt_ctrl_c_e2e");
    h.send_line("exit");
}
