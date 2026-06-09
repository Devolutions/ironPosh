//! Same-client disconnect → reconnect e2e through the interactive REPL.
//!
//! Requires the parallel session loop (`--parallel`): the serial loop keeps a
//! single request in flight and rejects disconnect/reconnect operations.

mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn parallel_tokio_client_disconnect_reconnect_preserves_state() {
    const MARKER: &str = "__PTY_DISCONNECT_RECONNECT__";

    let mut h = PtyHarness::try_spawn_tokio_client_with_args(&["--parallel"]);
    h.sleep_for_connect();

    // 1) Set a variable in the runspace.
    h.send_line("$x = 42");
    assert!(
        h.wait_for_output_contains(">", Duration::from_secs(20)),
        "did not observe prompt after setting variable. tail={}",
        h.tail_string(16 * 1024)
    );

    // 2) Disconnect the runspace pool; the REPL prints the shell id.
    h.send_line(":disconnect");
    assert!(
        h.wait_for_output_contains(
            "Disconnected from runspace pool (ShellId:",
            Duration::from_secs(30)
        ),
        "did not observe disconnect confirmation. tail={}",
        h.tail_string(16 * 1024)
    );

    // 3) Reconnect.
    h.send_line(":reconnect");
    assert!(
        h.wait_for_output_contains(
            "Reconnected to runspace pool (ShellId:",
            Duration::from_secs(30)
        ),
        "did not observe reconnect confirmation. tail={}",
        h.tail_string(16 * 1024)
    );

    // 4) The runspace state must have survived the disconnect.
    h.send_line(&format!("Write-Output \"{MARKER}=$x\""));
    assert!(
        h.wait_for_output_contains(&format!("{MARKER}=42"), Duration::from_secs(30)),
        "runspace state was not preserved across disconnect/reconnect. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
