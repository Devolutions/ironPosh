mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

#[test]
#[ignore = "stress e2e: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_terminal_stress_ctrl_c_hostcalls_and_output() {
    const WARM: &str = "__PTY_STRESS_WARMUP__";
    const DONE: &str = "__PTY_STRESS_DONE__";

    // Tweakable knobs (keep defaults conservative; this runs against a real server).
    //
    // - IRONPOSH_STRESS_ROUNDS: number of ping+cancel rounds (default 3)
    // - IRONPOSH_STRESS_PING_SECS: seconds to let ping run (default 2)
    // - IRONPOSH_STRESS_CTRL_C_BURST: ctrl+c presses per round (default 15)
    // - IRONPOSH_STRESS_CTRL_C_GAP_MS: gap between ctrl+c presses (default 20ms)
    // - IRONPOSH_STRESS_HOSTCALL_LINES: hostcall WriteLine count per round (default 50)
    // - IRONPOSH_STRESS_WAIT_SECS: per-step wait timeout (default 60)
    let rounds = env_usize("IRONPOSH_STRESS_ROUNDS", 3);
    let ping_secs = env_u64("IRONPOSH_STRESS_PING_SECS", 2);
    let ctrl_c_burst = env_usize("IRONPOSH_STRESS_CTRL_C_BURST", 15);
    let ctrl_c_gap_ms = env_u64("IRONPOSH_STRESS_CTRL_C_GAP_MS", 20);
    let hostcall_lines = env_usize("IRONPOSH_STRESS_HOSTCALL_LINES", 50);
    let wait_secs = env_u64("IRONPOSH_STRESS_WAIT_SECS", 60);

    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // Warm-up: make sure the session is alive before we start stressing it.
    h.send_line(&format!("Write-Output '{WARM}'"));
    assert!(
        h.wait_for_output_contains(WARM, Duration::from_secs(wait_secs)),
        "warm-up marker not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    for i in 1..=rounds {
        // 1) Long-running noisy command.
        h.send_line("ping 8.8.8.8 -n 1000");
        std::thread::sleep(Duration::from_secs(ping_secs));

        // 2) Cancel storm.
        h.send_ctrl_c_burst(ctrl_c_burst, Duration::from_millis(ctrl_c_gap_ms));
        std::thread::sleep(Duration::from_millis(400));

        // 3) Prove the session is still alive after cancel.
        let after_ping = format!("__PTY_STRESS_AFTER_PING_{i}__");
        h.send_line(&format!("whoami; Write-Output '{after_ping}'"));
        assert!(
            h.wait_for_output_contains(&after_ping, Duration::from_secs(wait_secs)),
            "after-ping marker not observed at round={i}. tail={}",
            h.tail_string(16 * 1024)
        );

        // 4) Hostcalls pressure (lots of UI writes) + pipeline output.
        let hc_done = format!("__PTY_STRESS_HOSTCALLS_{i}_DONE__");
        h.send_line(&format!(
            "$m='__PTY_STRESS_HOSTCALLS_{i}__'; \
             1..{hostcall_lines} | % {{ $Host.UI.WriteLine(\"$m\" + $_) }}; \
             Write-Output '{hc_done}'"
        ));
        assert!(
            h.wait_for_output_contains(&hc_done, Duration::from_secs(wait_secs)),
            "hostcalls done marker not observed at round={i}. tail={}",
            h.tail_string(16 * 1024)
        );

        // 5) Clear-Host should not crash (exercise RawUI buffer ops).
        let clear_done = format!("__PTY_STRESS_CLEAR_{i}_DONE__");
        h.send_line(&format!("Clear-Host; Write-Output '{clear_done}'"));
        assert!(
            h.wait_for_output_contains(&clear_done, Duration::from_secs(wait_secs)),
            "clear-host done marker not observed at round={i}. tail={}",
            h.tail_string(16 * 1024)
        );
    }

    // Final sanity marker.
    h.send_line(&format!("Write-Output '{DONE}'"));
    assert!(
        h.wait_for_output_contains(DONE, Duration::from_secs(wait_secs)),
        "final marker not observed. tail={}",
        h.tail_string(16 * 1024)
    );

    h.send_line("exit");
}
