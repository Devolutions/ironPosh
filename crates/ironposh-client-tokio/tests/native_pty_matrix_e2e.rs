mod support;

use std::time::Duration;

use support::native_pty_matrix::{
    run_native_alignment_cases, MatrixCase, MatrixStep, NativeTransport,
};

#[test]
#[ignore = "e2e test: requires native Windows PSRemoting endpoint from Enable-PSRemoting -Force"]
fn native_pty_matrix_commands_and_records() {
    run_native_alignment_cases(&[
        MatrixCase {
            id: "http_basic_order",
            category: "commands",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "basic-order",
                    "$p='__NPTY_BASIC_'; Write-Output ($p + 'ONE__'); Write-Output ($p + 'TWO__')",
                ),
                wait("basic-done", "__NPTY_BASIC_TWO__", 35),
            ],
            expect_contains: vec!["__NPTY_BASIC_ONE__", "__NPTY_BASIC_TWO__"],
            expect_absent: vec![],
            expect_order: vec!["__NPTY_BASIC_ONE__", "__NPTY_BASIC_TWO__"],
        },
        MatrixCase {
            id: "http_large_delayed_output",
            category: "commands",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "large-output",
                    "$p='__NPTY_LARGE_'; 1..200 | ForEach-Object { Write-Output ($p + $_ + '__') }; Start-Sleep -Milliseconds 150; Write-Output ($p + 'DONE__')",
                ),
                wait("large-done", "__NPTY_LARGE_DONE__", 45),
            ],
            expect_contains: vec![
                "__NPTY_LARGE_1__",
                "__NPTY_LARGE_100__",
                "__NPTY_LARGE_200__",
                "__NPTY_LARGE_DONE__",
            ],
            expect_absent: vec![],
            expect_order: vec![
                "__NPTY_LARGE_1__",
                "__NPTY_LARGE_100__",
                "__NPTY_LARGE_200__",
                "__NPTY_LARGE_DONE__",
            ],
        },
        MatrixCase {
            id: "http_native_command_exit_code",
            category: "commands",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "native-exit",
                    "$p='__NPTY_NATIVE_'; cmd.exe /c exit 7; Write-Output ($p + 'LASTEXIT__=' + $LASTEXITCODE)",
                ),
                wait("native-exit-done", "__NPTY_NATIVE_LASTEXIT__=7", 35),
            ],
            expect_contains: vec!["__NPTY_NATIVE_LASTEXIT__=7"],
            expect_absent: vec![],
            expect_order: vec!["__NPTY_NATIVE_LASTEXIT__=7"],
        },
        MatrixCase {
            id: "http_format_table",
            category: "commands",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "format-table",
                    "$p='__NPTY_TABLE_'; Get-Process -Id $PID | Select-Object Id,ProcessName | Format-Table -AutoSize; Write-Output ($p + 'DONE__')",
                ),
                wait("table-done", "__NPTY_TABLE_DONE__", 35),
            ],
            expect_contains: vec!["ProcessName", "__NPTY_TABLE_DONE__"],
            expect_absent: vec![],
            expect_order: vec!["ProcessName", "__NPTY_TABLE_DONE__"],
        },
    ]);
}

#[test]
#[ignore = "e2e test: requires native Windows PSRemoting endpoint from Enable-PSRemoting -Force"]
fn native_pty_matrix_streams_and_host_calls() {
    run_native_alignment_cases(&[
        MatrixCase {
            id: "http_stream_records",
            category: "streams",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "stream-records",
                    "$p='__NPTY_STREAM_'; $WarningPreference='Continue'; $VerbosePreference='Continue'; $DebugPreference='Continue'; $InformationPreference='Continue'; Write-Output ($p + 'OUT__'); Write-Warning ($p + 'WARN__'); Write-Error ($p + 'ERR__'); Write-Verbose ($p + 'VERBOSE__'); Write-Debug ($p + 'DEBUG__'); Write-Information ($p + 'INFO__') -InformationAction Continue; 1..3 | ForEach-Object { Write-Progress -Activity ($p + 'PROGRESS__') -Status $_ -PercentComplete ($_ * 30); Start-Sleep -Milliseconds 50 }; Write-Output ($p + 'DONE__')",
                ),
                wait("stream-done", "__NPTY_STREAM_DONE__", 45),
            ],
            expect_contains: vec![
                "__NPTY_STREAM_OUT__",
                "__NPTY_STREAM_WARN__",
                "__NPTY_STREAM_ERR__",
                "__NPTY_STREAM_VERBOSE__",
                "__NPTY_STREAM_DEBUG__",
                "__NPTY_STREAM_INFO__",
                "__NPTY_STREAM_PROGRESS__",
                "__NPTY_STREAM_DONE__",
            ],
            expect_absent: vec![],
            expect_order: vec!["__NPTY_STREAM_OUT__", "__NPTY_STREAM_DONE__"],
        },
        MatrixCase {
            id: "http_host_ui_rawui",
            category: "host-calls",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "host-ui",
                    "$p='__NPTY_HOST_'; Write-Output ($p + 'NAME__=' + $Host.Name); Write-Output ($p + 'VER__=' + $Host.Version.ToString()); $Host.UI.Write($p + 'WRITE__'); $Host.UI.WriteLine($p + 'WRITELN__'); $Host.UI.Write([System.ConsoleColor]::Red,[System.ConsoleColor]::DarkBlue,($p + 'COLOR__')); $Host.UI.WriteLine(''); Write-Output ($p + 'SIZE__=' + $Host.UI.RawUI.WindowSize.Width + 'x' + $Host.UI.RawUI.WindowSize.Height)",
                ),
                wait("host-size", "__NPTY_HOST_SIZE__=", 45),
            ],
            expect_contains: vec![
                "__NPTY_HOST_NAME__=",
                "__NPTY_HOST_VER__=",
                "__NPTY_HOST_WRITE__",
                "__NPTY_HOST_WRITELN__",
                "__NPTY_HOST_COLOR__",
                "__NPTY_HOST_SIZE__=",
            ],
            expect_absent: vec![],
            expect_order: vec!["__NPTY_HOST_NAME__=", "__NPTY_HOST_SIZE__="],
        },
        MatrixCase {
            id: "http_host_clear",
            category: "host-calls",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "host-clear",
                    "$p='__NPTY_CLEAR_'; Clear-Host; Write-Output ($p + 'AFTER__')",
                ),
                wait("host-clear-after", "__NPTY_CLEAR_AFTER__", 45),
            ],
            expect_contains: vec!["__NPTY_CLEAR_AFTER__"],
            expect_absent: vec![],
            expect_order: vec!["__NPTY_CLEAR_AFTER__"],
        },
    ]);
}

#[test]
#[ignore = "e2e test: requires native Windows PSRemoting endpoint from Enable-PSRemoting -Force"]
fn native_pty_matrix_interactive_operations() {
    run_native_alignment_cases(&[
        MatrixCase {
            id: "http_read_host",
            category: "interactive",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "read-host",
                    "$prompt='__NPTY_READ_' + 'PROMPT__'; $x = Read-Host $prompt; Write-Output (('__NPTY_READ_' + 'VALUE__=') + $x)",
                ),
                wait("read-host-prompt", "__NPTY_READ_PROMPT__", 35),
                line("read-host-input", "native-user"),
                wait("read-host-value", "__NPTY_READ_VALUE__=native-user", 35),
            ],
            expect_contains: vec!["__NPTY_READ_PROMPT__", "__NPTY_READ_VALUE__=native-user"],
            expect_absent: vec![],
            expect_order: vec!["__NPTY_READ_PROMPT__", "__NPTY_READ_VALUE__=native-user"],
        },
        MatrixCase {
            id: "http_secure_read_host",
            category: "interactive",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "secure-read-host",
                    "$prompt='__NPTY_SECURE_' + 'PROMPT__'; $s = Read-Host $prompt -AsSecureString; $b = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($s); try { $v = [Runtime.InteropServices.Marshal]::PtrToStringUni($b) } finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($b) }; Write-Output (('__NPTY_SECURE_' + 'VALUE__=') + $v)",
                ),
                wait("secure-read-host-prompt", "__NPTY_SECURE_PROMPT__", 35),
                line("secure-read-host-input", "sensitive-input"),
                wait(
                    "secure-read-host-value",
                    "__NPTY_SECURE_VALUE__=sensitive-input",
                    35,
                ),
            ],
            expect_contains: vec![
                "__NPTY_SECURE_PROMPT__",
                "__NPTY_SECURE_VALUE__=sensitive-input",
            ],
            expect_absent: vec![],
            expect_order: vec![
                "__NPTY_SECURE_PROMPT__",
                "__NPTY_SECURE_VALUE__=sensitive-input",
            ],
        },
        MatrixCase {
            id: "http_ctrl_c_long_command",
            category: "operations",
            transport: NativeTransport::Http,
            steps: vec![
                line(
                    "start-long-command",
                    "$p='__NPTY_CTRL_'; Write-Output ($p + 'START__'); Start-Sleep -Seconds 20; Write-Output ($p + 'SHOULD_NOT__')",
                ),
                wait("ctrl-start", "__NPTY_CTRL_START__", 35),
                bytes("ctrl-c", &[0x03]),
                sleep("after-ctrl-c", 1_000),
                line(
                    "after-ctrl-c",
                    "$p='__NPTY_CTRL_'; Write-Output ($p + 'AFTER__')",
                ),
                wait("ctrl-after", "__NPTY_CTRL_AFTER__", 35),
            ],
            expect_contains: vec!["__NPTY_CTRL_START__", "__NPTY_CTRL_AFTER__"],
            expect_absent: vec!["__NPTY_CTRL_SHOULD_NOT__"],
            expect_order: vec!["__NPTY_CTRL_START__", "__NPTY_CTRL_AFTER__"],
        },
        MatrixCase {
            id: "http_tab_completion",
            category: "operations",
            transport: NativeTransport::Http,
            steps: vec![
                bytes("type-partial-command", b"Get-Ser"),
                bytes("tab-complete", b"\t"),
                wait("tab-expanded", "Get-Service", 15),
                bytes("cancel-completed-command", &[0x03]),
                sleep("after-tab-cancel", 500),
                line(
                    "after-tab-completion",
                    "$p='__NPTY_TAB_'; Write-Output ($p + 'AFTER__')",
                ),
                wait("tab-after", "__NPTY_TAB_AFTER__", 35),
            ],
            expect_contains: vec!["Get-Service", "__NPTY_TAB_AFTER__"],
            expect_absent: vec![],
            expect_order: vec!["Get-Service", "__NPTY_TAB_AFTER__"],
        },
    ]);
}

#[test]
#[ignore = "e2e test: requires native Windows PSRemoting HTTPS endpoint and test certificate trust bypass"]
fn native_pty_matrix_https_insecure() {
    run_native_alignment_cases(&[MatrixCase {
        id: "https_insecure_basic",
        category: "ssl",
        transport: NativeTransport::HttpsInsecure,
        steps: vec![
            line(
                "https-basic",
                "$p='__NPTY_HTTPS_'; Write-Output ($p + 'BASIC__')",
            ),
            wait("https-basic-done", "__NPTY_HTTPS_BASIC__", 45),
        ],
        expect_contains: vec!["__NPTY_HTTPS_BASIC__"],
        expect_absent: vec![],
        expect_order: vec!["__NPTY_HTTPS_BASIC__"],
    }]);
}

fn line(label: &'static str, text: &'static str) -> MatrixStep {
    MatrixStep::Line { label, text }
}

fn bytes(label: &'static str, bytes: &'static [u8]) -> MatrixStep {
    MatrixStep::Bytes { label, bytes }
}

fn wait(label: &'static str, text: &'static str, timeout_secs: u64) -> MatrixStep {
    MatrixStep::WaitFor {
        label,
        text,
        timeout: Duration::from_secs(timeout_secs),
    }
}

fn sleep(label: &'static str, millis: u64) -> MatrixStep {
    MatrixStep::Sleep {
        label,
        duration: Duration::from_millis(millis),
    }
}
