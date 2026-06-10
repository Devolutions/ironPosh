#![allow(dead_code)]

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use serde_json::json;
use std::{
    collections::VecDeque,
    ffi::OsStr,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const ROWS: u16 = 30;
const COLS: u16 = 120;
const SCROLLBACK: usize = 10_000;
const OUTPUT_CAP_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyRole {
    Native,
    Tokio,
}

impl PtyRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Tokio => "tokio",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTransport {
    Http,
    HttpsInsecure,
}

#[derive(Debug, Clone)]
pub struct NativeEndpointConfig {
    pub server: String,
    pub http_port: u16,
    pub https_port: u16,
    pub username: String,
    pub password: String,
}

impl NativeEndpointConfig {
    pub fn load() -> Self {
        let dev_env = load_dev_env_native_defaults();
        Self {
            server: first_env("IRONPOSH_NATIVE_SERVER")
                .or_else(|| dev_env.as_ref().and_then(|d| d.server.clone()))
                .unwrap_or_else(|| "10.10.0.3".to_string()),
            http_port: first_env("IRONPOSH_NATIVE_HTTP_PORT")
                .and_then(|v| v.parse().ok())
                .unwrap_or(5985),
            https_port: first_env("IRONPOSH_NATIVE_HTTPS_PORT")
                .and_then(|v| v.parse().ok())
                .unwrap_or(5986),
            username: first_env("IRONPOSH_NATIVE_USERNAME")
                .or_else(|| dev_env.as_ref().and_then(|d| d.username.clone()))
                .unwrap_or_else(|| "administrator".to_string()),
            password: first_env("IRONPOSH_NATIVE_PASSWORD")
                .or_else(|| dev_env.as_ref().and_then(|d| d.password.clone()))
                .expect(
                    "IRONPOSH_NATIVE_PASSWORD or commands.native password in .dev_env.json is required",
                ),
        }
    }

    fn port_for(&self, transport: NativeTransport) -> u16 {
        match transport {
            NativeTransport::Http => self.http_port,
            NativeTransport::HttpsInsecure => self.https_port,
        }
    }
}

#[derive(Debug, Clone)]
struct DevEnvNativeDefaults {
    server: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MatrixStep {
    Line {
        label: &'static str,
        text: &'static str,
    },
    Bytes {
        label: &'static str,
        bytes: &'static [u8],
    },
    WaitFor {
        label: &'static str,
        text: &'static str,
        timeout: Duration,
    },
    Sleep {
        label: &'static str,
        duration: Duration,
    },
}

#[derive(Debug, Clone)]
pub struct MatrixCase {
    pub id: &'static str,
    pub category: &'static str,
    pub transport: NativeTransport,
    pub steps: Vec<MatrixStep>,
    pub expect_contains: Vec<&'static str>,
    pub expect_absent: Vec<&'static str>,
    pub expect_order: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct RecordedInput {
    label: String,
    kind: String,
    value: String,
    elapsed_ms: u128,
}

#[derive(Debug, Clone)]
pub struct PtyRecording {
    pub role: PtyRole,
    pub case_id: String,
    pub raw_output: Vec<u8>,
    pub screen_history_output: Vec<u8>,
    pub inputs: Vec<RecordedInput>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone)]
pub struct NormalizedPty {
    pub plain_text: String,
    pub screen_text: String,
    pub screen_history_text: String,
    pub cursor: (u16, u16),
}

impl PtyRecording {
    pub fn normalize(&self) -> NormalizedPty {
        let plain_text = normalize_text(&strip_ansi(&self.raw_output));
        let screen_history = String::from_utf8_lossy(&self.screen_history_output);
        let screen_history_text = normalize_text(&screen_history);
        let mut parser = vt100::Parser::new(ROWS, COLS, SCROLLBACK);
        parser.process(&self.raw_output);
        let screen = parser.screen();
        NormalizedPty {
            plain_text,
            screen_text: normalize_text(&screen.contents()),
            screen_history_text,
            cursor: screen.cursor_position(),
        }
    }

    pub fn tail(&self, max_bytes: usize) -> String {
        let start = self.raw_output.len().saturating_sub(max_bytes);
        String::from_utf8_lossy(&self.raw_output[start..]).into_owned()
    }
}

pub fn run_native_alignment_cases(cases: &[MatrixCase]) {
    let cfg = NativeEndpointConfig::load();
    for case in cases {
        run_native_alignment_case(&cfg, case);
    }
}

fn run_native_alignment_case(cfg: &NativeEndpointConfig, case: &MatrixCase) {
    let root = artifact_root(case);
    let native = drive_role(cfg, case, PtyRole::Native, &root);
    let tokio = drive_role(cfg, case, PtyRole::Tokio, &root);
    assert_recording_observations(case, &native);
    assert_recording_observations(case, &tokio);
    assert_recordings_align(case, &native, &tokio);
}

fn drive_role(
    cfg: &NativeEndpointConfig,
    case: &MatrixCase,
    role: PtyRole,
    artifact_root: &Path,
) -> PtyRecording {
    let mut session = PtyRecorder::spawn(cfg, case, role, artifact_root);
    let ready = format!("__NPTY_READY_{}_{}__", case.id, role.as_str());
    let mut failure = session.wait_until_ready(cfg, case, &ready);

    if failure.is_none() {
        for step in &case.steps {
            if let Some(reason) = session.apply_step(step) {
                failure = Some(reason);
                break;
            }
        }
    }

    session.send_line("exit", "exit");
    std::thread::sleep(Duration::from_millis(500));
    let recording = session.recording();
    let normalized = recording.normalize();
    let path = save_artifacts(artifact_root, &recording, &normalized, cfg, case);

    if let Some(reason) = failure {
        panic!(
            "{role:?} failed case {}: {reason}\nartifacts={}\ntail={}",
            case.id,
            path.display(),
            recording.tail(16 * 1024)
        );
    }

    recording
}

struct PtyRecorder {
    role: PtyRole,
    case_id: String,
    out: Arc<Mutex<VecDeque<u8>>>,
    screen_history: Arc<Mutex<VecDeque<u8>>>,
    inputs: Vec<RecordedInput>,
    start: Instant,
    _master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Option<Box<dyn portable_pty::Child + Send>>,
}

impl PtyRecorder {
    fn spawn(
        cfg: &NativeEndpointConfig,
        case: &MatrixCase,
        role: PtyRole,
        artifact_root: &Path,
    ) -> Self {
        let pty_system = native_pty_system();
        let pair: PtyPair = pty_system
            .openpty(PtySize {
                rows: ROWS,
                cols: COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("open pty");

        let cmd = match role {
            PtyRole::Native => native_command(cfg, case.transport),
            PtyRole::Tokio => tokio_command(cfg, case, artifact_root),
        };

        let child = pair.slave.spawn_command(cmd).expect("spawn pty command");
        let master = pair.master;
        let writer = master.take_writer().expect("take pty writer");
        let mut reader = master.try_clone_reader().expect("clone pty reader");
        let out = Arc::new(Mutex::new(VecDeque::new()));
        let out_reader = Arc::clone(&out);
        let screen_history = Arc::new(Mutex::new(VecDeque::new()));
        let screen_history_reader = Arc::clone(&screen_history);

        let _reader_handle = std::thread::spawn(move || {
            let mut tmp = [0_u8; 8192];
            let mut parser = vt100::Parser::new(ROWS, COLS, SCROLLBACK);
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut guard = out_reader.lock().expect("lock output");
                        push_capped(&mut guard, OUTPUT_CAP_BYTES, &tmp[..n]);
                        drop(guard);

                        parser.process(&tmp[..n]);
                        let screen_text = normalize_text(&parser.screen().contents());
                        if !screen_text.trim().is_empty() {
                            let mut guard =
                                screen_history_reader.lock().expect("lock screen history");
                            push_capped(&mut guard, OUTPUT_CAP_BYTES, screen_text.as_bytes());
                            push_capped(&mut guard, OUTPUT_CAP_BYTES, b"\n");
                        }
                    }
                }
            }
        });

        Self {
            role,
            case_id: case.id.to_string(),
            out,
            screen_history,
            inputs: Vec::new(),
            start: Instant::now(),
            _master: master,
            writer,
            child: Some(child),
        }
    }

    fn wait_until_ready(
        &mut self,
        cfg: &NativeEndpointConfig,
        case: &MatrixCase,
        ready: &str,
    ) -> Option<String> {
        match self.role {
            PtyRole::Native => {
                let prompt_hint = format!("[{}]", cfg.server);
                if !self.wait_for_output_contains(&prompt_hint, Duration::from_secs(35)) {
                    return Some(format!(
                        "native Enter-PSSession prompt missing: {prompt_hint}"
                    ));
                }
            }
            PtyRole::Tokio => std::thread::sleep(Duration::from_secs(8)),
        }

        self.send_line(&format!("Write-Output '{ready}'"), "ready");
        if self.wait_for_output_contains(ready, Duration::from_secs(35)) {
            None
        } else {
            Some(format!("ready marker missing for {}", case.id))
        }
    }

    fn apply_step(&mut self, step: &MatrixStep) -> Option<String> {
        match step {
            MatrixStep::Line { label, text } => {
                self.send_line(text, label);
                None
            }
            MatrixStep::Bytes { label, bytes } => {
                self.send_bytes(bytes, label);
                None
            }
            MatrixStep::WaitFor {
                label,
                text,
                timeout,
            } => {
                if self.wait_for_output_contains(text, *timeout) {
                    None
                } else {
                    Some(format!("wait step {label} timed out for text {text}"))
                }
            }
            MatrixStep::Sleep { label: _, duration } => {
                std::thread::sleep(*duration);
                None
            }
        }
    }

    fn send_line(&mut self, text: &str, label: &str) {
        self.record_input(label, "line", text);
        write_all_paced(&mut self.writer, text.as_bytes()).expect("write line");
        write_all_paced(&mut self.writer, b"\r").expect("write carriage return");
        self.writer.flush().expect("flush line");
    }

    fn send_bytes(&mut self, bytes: &[u8], label: &str) {
        self.record_input(label, "bytes", &hex_bytes(bytes));
        self.writer.write_all(bytes).expect("write bytes");
        self.writer.flush().expect("flush bytes");
    }

    fn record_input(&mut self, label: &str, kind: &str, value: &str) {
        self.inputs.push(RecordedInput {
            label: label.to_string(),
            kind: kind.to_string(),
            value: value.to_string(),
            elapsed_ms: self.start.elapsed().as_millis(),
        });
    }

    fn wait_for_output_contains(&self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let has = {
                let guard = self.out.lock().expect("lock output");
                let screen_history = self.screen_history.lock().expect("lock screen history");
                buffer_contains(&guard, needle.as_bytes())
                    || buffer_contains(&screen_history, needle.as_bytes())
            };
            if has {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn recording(&self) -> PtyRecording {
        let raw_output = {
            let guard = self.out.lock().expect("lock output");
            guard.iter().copied().collect()
        };
        let screen_history_output = {
            let guard = self.screen_history.lock().expect("lock screen history");
            guard.iter().copied().collect()
        };
        PtyRecording {
            role: self.role,
            case_id: self.case_id.clone(),
            raw_output,
            screen_history_output,
            inputs: self.inputs.clone(),
            elapsed_ms: self.start.elapsed().as_millis(),
        }
    }
}

impl Drop for PtyRecorder {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

fn native_command(cfg: &NativeEndpointConfig, transport: NativeTransport) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("powershell.exe");
    cmd.arg("-NoLogo");
    cmd.arg("-NoProfile");
    cmd.arg("-NoExit");
    cmd.arg("-Command");
    cmd.env("IRONPOSH_NATIVE_PASSWORD", OsStr::new(&cfg.password));

    let use_ssl = if transport == NativeTransport::HttpsInsecure {
        format!(" -UseSSL -Port {}", cfg.https_port)
    } else {
        String::new()
    };
    let session_options = if transport == NativeTransport::HttpsInsecure {
        "New-PSSessionOption -NoCompression -SkipCACheck -SkipCNCheck -SkipRevocationCheck"
    } else {
        "New-PSSessionOption -NoCompression"
    };
    let script = format!(
        "$sec = ConvertTo-SecureString $env:IRONPOSH_NATIVE_PASSWORD -AsPlainText -Force; \
         $cred = [System.Management.Automation.PSCredential]::new('{}', $sec); \
         Enter-PSSession -ComputerName '{}' -Authentication Basic -Credential $cred{use_ssl} \
         -SessionOption ({session_options})",
        ps_quote(&cfg.username),
        ps_quote(&cfg.server)
    );
    cmd.arg(script);
    cmd
}

fn tokio_command(
    cfg: &NativeEndpointConfig,
    case: &MatrixCase,
    artifact_root: &Path,
) -> CommandBuilder {
    let bin = env!("CARGO_BIN_EXE_ironposh-client-tokio");
    let mut cmd = CommandBuilder::new(bin);
    let port = cfg.port_for(case.transport);
    let log_file = artifact_root.join("tokio.log");
    cmd.env("IRONPOSH_TOKIO_LOG_FILE", log_file.as_os_str());
    cmd.arg("--server");
    cmd.arg(&cfg.server);
    cmd.arg("--port");
    cmd.arg(port.to_string());
    cmd.arg("--username");
    cmd.arg(&cfg.username);
    cmd.arg("--password");
    cmd.arg(&cfg.password);
    cmd.arg("--auth-method");
    cmd.arg("basic");
    if case.transport == NativeTransport::HttpsInsecure {
        cmd.arg("--https");
        cmd.arg("--insecure");
    }
    cmd
}

fn assert_recording_observations(case: &MatrixCase, recording: &PtyRecording) {
    let normalized = recording.normalize();
    for expected in &case.expect_contains {
        assert!(
            normalized_contains(&normalized, expected),
            "{:?} case {} missing expected text {expected}\nobservation_tail={}\nscreen={}",
            recording.role,
            case.id,
            observation_tail(&normalized, 4000),
            normalized.screen_text
        );
    }
    for absent in &case.expect_absent {
        assert!(
            !normalized_contains(&normalized, absent),
            "{:?} case {} contained forbidden text {absent}\nobservation_tail={}\nscreen={}",
            recording.role,
            case.id,
            observation_tail(&normalized, 4000),
            normalized.screen_text
        );
    }
    assert_order(case, recording.role, &normalized);
}

fn assert_recordings_align(case: &MatrixCase, native: &PtyRecording, tokio: &PtyRecording) {
    let native_norm = native.normalize();
    let tokio_norm = tokio.normalize();
    for expected in &case.expect_contains {
        let native_has = normalized_contains(&native_norm, expected);
        let tokio_has = normalized_contains(&tokio_norm, expected);
        assert_eq!(
            native_has,
            tokio_has,
            "case {} alignment mismatch for {expected}\nnative_tail={}\ntokio_tail={}",
            case.id,
            observation_tail(&native_norm, 4000),
            observation_tail(&tokio_norm, 4000)
        );
    }
}

fn assert_order(case: &MatrixCase, role: PtyRole, normalized: &NormalizedPty) {
    let ordered_text = observation_text(normalized);
    let mut cursor = 0;
    for token in &case.expect_order {
        let haystack = &ordered_text[cursor..];
        let Some(pos) = haystack.find(token) else {
            panic!(
                "{role:?} case {} missing ordered token {token}\nobservation_tail={}",
                case.id,
                plain_tail(&ordered_text, 4000)
            );
        };
        cursor += pos + token.len();
    }
}

fn normalized_contains(normalized: &NormalizedPty, needle: &str) -> bool {
    normalized.plain_text.contains(needle)
        || normalized.screen_text.contains(needle)
        || normalized.screen_history_text.contains(needle)
}

fn observation_text(normalized: &NormalizedPty) -> String {
    format!(
        "{}\n{}\n{}",
        normalized.plain_text, normalized.screen_history_text, normalized.screen_text
    )
}

fn observation_tail(normalized: &NormalizedPty, max_chars: usize) -> String {
    plain_tail(&observation_text(normalized), max_chars)
}

fn save_artifacts(
    root: &Path,
    recording: &PtyRecording,
    normalized: &NormalizedPty,
    cfg: &NativeEndpointConfig,
    case: &MatrixCase,
) -> PathBuf {
    let role_dir = root.join(recording.role.as_str());
    std::fs::create_dir_all(&role_dir).expect("create recording artifact dir");
    std::fs::write(role_dir.join("raw.bin"), &recording.raw_output).expect("write raw pty bytes");
    std::fs::write(role_dir.join("plain.txt"), &normalized.plain_text)
        .expect("write plain pty text");
    std::fs::write(role_dir.join("screen.txt"), &normalized.screen_text)
        .expect("write screen pty text");
    std::fs::write(
        role_dir.join("screen_history.txt"),
        &normalized.screen_history_text,
    )
    .expect("write screen history pty text");

    let inputs = recording
        .inputs
        .iter()
        .map(|input| {
            json!({
                "label": input.label,
                "kind": input.kind,
                "value": input.value,
                "elapsed_ms": input.elapsed_ms,
            })
        })
        .collect::<Vec<_>>();
    let metadata = json!({
        "case_id": case.id,
        "category": case.category,
        "role": recording.role.as_str(),
        "transport": format!("{:?}", case.transport),
        "server": cfg.server,
        "http_port": cfg.http_port,
        "https_port": cfg.https_port,
        "username": cfg.username,
        "password": "<redacted>",
        "elapsed_ms": recording.elapsed_ms,
        "cursor": [normalized.cursor.0, normalized.cursor.1],
        "inputs": inputs,
        "unix_time": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });
    std::fs::write(
        role_dir.join("metadata.json"),
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");
    role_dir
}

fn artifact_root(case: &MatrixCase) -> PathBuf {
    workspace_root()
        .join("target")
        .join("ironposh-native-pty-recordings")
        .join(case.id)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn load_dev_env_native_defaults() -> Option<DevEnvNativeDefaults> {
    let path = workspace_root().join(".dev_env.json");
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let native = value.get("commands")?.get("native")?.as_str()?;
    Some(DevEnvNativeDefaults {
        server: extract_after(native, "-ComputerName"),
        username: extract_between(native, "PSCredential('", "'"),
        password: extract_between(native, "ConvertTo-SecureString '", "'"),
    })
}

fn first_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_after(text: &str, flag: &str) -> Option<String> {
    let rest = text.split_once(flag)?.1.trim_start();
    rest.split_whitespace()
        .next()
        .map(|value| value.trim_matches('"').trim_matches('\'').to_string())
}

fn extract_between(text: &str, start: &str, end: &str) -> Option<String> {
    let rest = text.split_once(start)?.1;
    Some(rest.split_once(end)?.0.to_string())
}

fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn push_capped(buf: &mut VecDeque<u8>, cap: usize, bytes: &[u8]) {
    for &b in bytes {
        if buf.len() == cap {
            buf.pop_front();
        }
        buf.push_back(b);
    }
}

fn write_all_paced(writer: &mut (dyn Write + Send), bytes: &[u8]) -> std::io::Result<()> {
    for chunk in bytes.chunks(16) {
        writer.write_all(chunk)?;
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn buffer_contains(haystack: &VecDeque<u8>, needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    let h: Vec<u8> = haystack.iter().copied().collect();
    h.windows(needle.len()).any(|window| window == needle)
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\0', "")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_ansi(bytes: &[u8]) -> String {
    #[derive(Debug, Clone, Copy)]
    enum State {
        Ground,
        Escape,
        Csi,
        Osc,
        OscEscape,
    }

    let mut out = Vec::with_capacity(bytes.len());
    let mut state = State::Ground;
    for &byte in bytes {
        match state {
            State::Ground => {
                if byte == 0x1b {
                    state = State::Escape;
                } else {
                    out.push(byte);
                }
            }
            State::Escape => match byte {
                b'[' => state = State::Csi,
                b']' => state = State::Osc,
                0x1b => state = State::Escape,
                _ => state = State::Ground,
            },
            State::Csi => {
                if (0x40..=0x7e).contains(&byte) {
                    state = State::Ground;
                }
            }
            State::Osc => match byte {
                0x07 => state = State::Ground,
                0x1b => state = State::OscEscape,
                _ => {}
            },
            State::OscEscape => {
                state = if byte == b'\\' {
                    State::Ground
                } else {
                    State::Osc
                };
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn plain_tail(text: &str, max_chars: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}
