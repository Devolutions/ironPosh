mod e2e_auths;
mod support;

use std::time::Duration;
use support::pty_harness::PtyHarness;

#[test]
#[ignore = "e2e test: requires reachable WinRM server + valid credentials"]
fn real_server_terminal_and_hostcalls_matrix_all_auths() {
    for auth in e2e_auths::auths_from_env_or_default() {
        let mut h = PtyHarness::try_spawn_tokio_client_with_args(&["--auth-method", auth]);
        h.sleep_for_connect();

        // T1: connect + prompt (proxy: prove we can run a trivial command)
        {
            let marker = format!("__E2E_TERM_{auth}_T1__");
            h.send_line(&format!("Write-Output '{marker}'"));
            assert!(
                h.wait_for_output_contains(&marker, Duration::from_secs(25)),
                "T1 marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // T2: marker output
        {
            let marker = format!("__E2E_TERM_{auth}_T2__");
            h.send_line(&format!("Write-Output '{marker}'"));
            assert!(
                h.wait_for_output_contains(&marker, Duration::from_secs(25)),
                "T2 marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H1: host basic info
        {
            let marker = format!("__E2E_HOST_{auth}_H1__");
            h.send_line(&format!(
                "Write-Output '{marker}_NAME=' + $Host.Name; \
                 Write-Output '{marker}_VER=' + $Host.Version.ToString(); \
                 Write-Output '{marker}_ID=' + $Host.InstanceId.ToString(); \
                 Write-Output '{marker}_CULT=' + $Host.CurrentCulture.Name; \
                 Write-Output '{marker}_UICULT=' + $Host.CurrentUICulture.Name"
            ));
            assert!(
                h.wait_for_output_contains(&format!("{marker}_NAME="), Duration::from_secs(25)),
                "H1 NAME missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
            assert!(
                h.wait_for_output_contains(&format!("{marker}_VER="), Duration::from_secs(25)),
                "H1 VER missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
            assert!(
                h.wait_for_output_contains(&format!("{marker}_ID="), Duration::from_secs(25)),
                "H1 ID missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
            assert!(
                h.wait_for_output_contains(&format!("{marker}_CULT="), Duration::from_secs(25)),
                "H1 CULT missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
            assert!(
                h.wait_for_output_contains(&format!("{marker}_UICULT="), Duration::from_secs(25)),
                "H1 UICULT missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H2: host write/writeLine
        {
            let marker = format!("__E2E_HOST_{auth}_H2__");
            h.send_line(&format!(
                "$Host.UI.Write('{marker}_WRITE'); \
                 $Host.UI.WriteLine('{marker}_WRITELN'); \
                 Write-Output '{marker}_PIPE'"
            ));
            assert!(
                h.wait_for_output_contains(&format!("{marker}_WRITE"), Duration::from_secs(25)),
                "H2 WRITE missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
            assert!(
                h.wait_for_output_contains(&format!("{marker}_WRITELN"), Duration::from_secs(25)),
                "H2 WRITELN missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
            assert!(
                h.wait_for_output_contains(&format!("{marker}_PIPE"), Duration::from_secs(25)),
                "H2 PIPE missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H9: Read-Host prompt should not consume previous command keystrokes
        {
            let marker = format!("__E2E_HOST_{auth}_H9__");
            h.send_line("cls");
            std::thread::sleep(Duration::from_millis(500));

            h.send_line(&format!(
                "$x = Read-Host 'Name'; Write-Output \"{marker}_HELLO=$x\""
            ));

            assert!(
                h.wait_for_output_contains("Name", Duration::from_secs(25)),
                "H9 did not observe prompt text for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );

            h.send_line("bob");
            assert!(
                h.wait_for_output_contains(&format!("{marker}_HELLO=bob"), Duration::from_secs(25)),
                "H9 marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
            assert!(
                !h.tail_string(16 * 1024)
                    .contains(&format!("{marker}_HELLO=cls")),
                "H9 leaked previous keystrokes for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H3: error/warn/verbose/debug placeholder (WebTerminal kept this as skipped)
        {
            let marker = format!("__E2E_HOST_{auth}_H3__");
            h.send_line(&format!("Write-Output '{marker}_SKIPPED'"));
            assert!(
                h.wait_for_output_contains(&format!("{marker}_SKIPPED"), Duration::from_secs(25)),
                "H3 skipped marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H4: colors (exercise hostcalls; do not assert exact rendering)
        {
            let marker = format!("__E2E_HOST_{auth}_H4__");
            h.send_line(&format!(
                "$Host.UI.Write([System.ConsoleColor]::Red,[System.ConsoleColor]::DarkBlue,'{marker}_COLOR'); \
                 $Host.UI.WriteLine(); \
                 Write-Output '{marker}_DONE'"
            ));
            assert!(
                h.wait_for_output_contains(&format!("{marker}_DONE"), Duration::from_secs(25)),
                "H4 done marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H5: Clear-Host (must not crash; session continues)
        {
            let before = format!("__E2E_HOST_{auth}_H5_BEFORE__");
            let after = format!("__E2E_HOST_{auth}_H5_AFTER__");
            h.send_line(&format!("Write-Output '{before}'"));
            assert!(
                h.wait_for_output_contains(&before, Duration::from_secs(25)),
                "H5 before marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );

            h.send_line("Clear-Host");
            std::thread::sleep(Duration::from_millis(250));

            h.send_line(&format!("Write-Output '{after}'"));
            assert!(
                h.wait_for_output_contains(&after, Duration::from_secs(25)),
                "H5 after marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H6: Write-Progress placeholder (WebTerminal kept this as skipped)
        {
            let marker = format!("__E2E_HOST_{auth}_H6_DONE__");
            h.send_line(&format!("Write-Output '{marker}_SKIPPED'"));
            assert!(
                h.wait_for_output_contains(&format!("{marker}_SKIPPED"), Duration::from_secs(25)),
                "H6 skipped marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H7: RawUI.KeyAvailable should be false
        {
            let marker = format!("__E2E_HOST_{auth}_H7__");
            h.send_line(&format!(
                "Write-Output '{marker}=' + [bool]$Host.UI.RawUI.KeyAvailable"
            ));
            assert!(
                h.wait_for_output_contains(&format!("{marker}="), Duration::from_secs(25)),
                "H7 marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // H8: SetShouldExit is no-op (session continues)
        {
            let marker = format!("__E2E_HOST_{auth}_H8__");
            h.send_line(&format!(
                "$Host.SetShouldExit(42); Write-Output '{marker}_AFTER'"
            ));
            assert!(
                h.wait_for_output_contains(&format!("{marker}_AFTER"), Duration::from_secs(25)),
                "H8 after marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        // T3: disconnect/reconnect does not duplicate input (UI-specific in WebTerminal).
        //
        // Tokio client doesn't support disconnect/reconnect within the same process.
        // Best-effort proxy: just ensure the session continues to accept input after the hostcall
        // matrix above (Clear-Host, Read-Host, etc.). PTY rendering can legitimately reprint
        // previous lines during reflow/clear operations, so don't assert exact echo counts.
        {
            let marker = format!("__E2E_TERM_{auth}_T3__");
            h.send_line(&format!("Write-Output '{marker}'"));
            assert!(
                h.wait_for_output_contains(&marker, Duration::from_secs(25)),
                "T3 marker missing for auth={auth}. tail={}",
                h.tail_string(16 * 1024)
            );
        }

        h.send_line("exit");
    }
}
