mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn serial_tokio_client_interactive_hostcalls_do_not_break_session() {
    let mut h = PtyHarness::try_spawn_tokio_client();
    h.sleep_for_connect();

    // 1) Read-Host
    {
        const MARKER: &str = "__PTY_INTERACTIVE_READ_HOST__";
        h.send_line(&format!(
            "$x = Read-Host \"Enter something\"; Write-Output \"{MARKER}=$x\""
        ));

        assert!(
            h.wait_for_output_contains("Enter something", Duration::from_secs(20)),
            "did not observe Read-Host prompt text. tail={}",
            h.tail_string(16 * 1024)
        );

        h.send_line("hello");

        assert!(
            h.wait_for_output_contains(&format!("{MARKER}=hello"), Duration::from_secs(20)),
            "did not observe Read-Host marker output. tail={}",
            h.tail_string(16 * 1024)
        );
    }

    // 2) PromptForChoice
    {
        const MARKER: &str = "__PTY_INTERACTIVE_CHOICE__";
        h.send_line(&format!(
            "$c = $host.UI.PromptForChoice(\"Title\",\"Message\",@(\"&Yes\",\"&No\"),0); Write-Output \"{MARKER}=$c\""
        ));

        assert!(
            h.wait_for_output_contains("Choice (default 0):", Duration::from_secs(20)),
            "did not observe PromptForChoice prompt. tail={}",
            h.tail_string(16 * 1024)
        );

        h.send_line("1");

        assert!(
            h.wait_for_output_contains(&format!("{MARKER}=1"), Duration::from_secs(20)),
            "did not observe PromptForChoice marker output. tail={}",
            h.tail_string(16 * 1024)
        );
    }

    // 3) Read-Host -AsSecureString (implementation varies; we just provide input once we see the prompt).
    {
        const MARKER: &str = "__PTY_INTERACTIVE_SECURESTRING__";
        h.send_line(&format!(
            "$s = Read-Host \"Password\" -AsSecureString; \
             $b = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($s); \
             try {{ $p = [Runtime.InteropServices.Marshal]::PtrToStringUni($b) }} finally {{ [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($b) }}; \
             Write-Output \"{MARKER}=$p\""
        ));

        // In most hosts, the prompt is written before the secure read occurs.
        // If we fail to observe it, still try sending input (best-effort).
        let _ = h.wait_for_output_contains("Password", Duration::from_secs(10));
        h.send_line("hunter2");

        assert!(
            h.wait_for_output_contains(&format!("{MARKER}=hunter2"), Duration::from_secs(25)),
            "did not observe SecureString marker output. tail={}",
            h.tail_string(16 * 1024)
        );
    }

    // Prove the session is still usable after interactive hostcalls.
    {
        const DONE: &str = "__PTY_INTERACTIVE_DONE__";
        h.send_line(&format!("Write-Output '{DONE}'"));
        assert!(
            h.wait_for_output_contains(DONE, Duration::from_secs(20)),
            "session did not produce final marker after interactive flows. tail={}",
            h.tail_string(16 * 1024)
        );
    }

    h.send_line("exit");
}
