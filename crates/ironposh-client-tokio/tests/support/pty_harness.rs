#![allow(dead_code)]

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::e2e_pwsh_config;

pub struct PtyHarness {
    pub out: Arc<Mutex<VecDeque<u8>>>,
    // Keep the PTY master alive for the lifetime of the harness.
    // On Windows/ConPTY, dropping the master can tear down the pseudo console,
    // closing the underlying pipes even if the cloned reader/writer handles are kept.
    _master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Option<Box<dyn portable_pty::Child + Send>>,
    log_file: std::path::PathBuf,
}

impl PtyHarness {
    pub fn try_spawn_tokio_client() -> Self {
        Self::try_spawn_tokio_client_with_args(&[])
    }

    pub fn try_spawn_tokio_client_with_args(extra_args: &[&str]) -> Self {
        let bin = env!("CARGO_BIN_EXE_ironposh-client-tokio");

        let cfg = e2e_pwsh_config::load_from_env_or_default();

        // Keep logs under the workspace so they are easy to inspect after failures.
        let log_file =
            std::path::PathBuf::from(r"D:\ironwinrm\logs\ironposh-client-tokio.pty-e2e.log");

        let pty_system = native_pty_system();
        let pair: PtyPair = pty_system
            .openpty(PtySize {
                rows: 30,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("open pty");

        let mut cmd = CommandBuilder::new(bin);
        cmd.env("IRONPOSH_TOKIO_LOG_FILE", log_file.as_os_str());

        cmd.arg("--server");
        cmd.arg(&cfg.hostname);
        cmd.arg("--port");
        cmd.arg(&cfg.port);
        cmd.arg("--username");
        cmd.arg(&cfg.username);
        cmd.arg("--password");
        cmd.arg(&cfg.password);
        if let Some(domain) = cfg.domain.as_deref() {
            cmd.arg("--domain");
            cmd.arg(domain);
        }

        for a in extra_args {
            cmd.arg(a);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .expect("spawn ironposh-client-tokio in pty");

        let master = pair.master;
        let writer = master.take_writer().expect("take pty writer");
        let mut reader = master.try_clone_reader().expect("clone pty reader");

        let out: Arc<Mutex<VecDeque<u8>>> = Arc::new(Mutex::new(VecDeque::new()));
        let out_reader = Arc::clone(&out);
        let _reader_handle = std::thread::spawn(move || {
            let mut tmp = [0u8; 8192];
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut guard = out_reader.lock().expect("lock output buffer");
                        push_capped(&mut guard, 1_000_000, &tmp[..n]);
                    }
                }
            }
        });

        Self {
            out,
            _master: master,
            writer,
            child: Some(child),
            log_file,
        }
    }

    #[allow(clippy::unused_self)]
    pub fn sleep_for_connect(&self) {
        // We don't have a stable prompt token because many prompts are hostcall-rendered.
        std::thread::sleep(Duration::from_secs(10));
    }

    pub fn send_line(&mut self, s: &str) {
        let mut child_exit_status = None;
        let mut child_pid = None;
        if let Some(child) = self.child.as_mut() {
            child_pid = child.process_id();
            child_exit_status = child.try_wait().ok().flatten();
        }

        if let Err(e) = self.writer.write_all(s.as_bytes()) {
            panic!(
                "write command bytes: {e:?}\nchild_pid={child_pid:?}\nchild_exit_status={child_exit_status:?}\nlog_file={}\npty_tail={}",
                self.log_file.display(),
                self.tail_string(16 * 1024)
            );
        }
        if let Err(e) = self.writer.write_all(b"\r") {
            panic!(
                "write carriage return: {e:?}\nchild_pid={child_pid:?}\nchild_exit_status={child_exit_status:?}\nlog_file={}\npty_tail={}",
                self.log_file.display(),
                self.tail_string(16 * 1024)
            );
        }
        if let Err(e) = self.writer.flush() {
            panic!(
                "flush command: {e:?}\nchild_pid={child_pid:?}\nchild_exit_status={child_exit_status:?}\nlog_file={}\npty_tail={}",
                self.log_file.display(),
                self.tail_string(16 * 1024)
            );
        }
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        let mut child_exit_status = None;
        let mut child_pid = None;
        if let Some(child) = self.child.as_mut() {
            child_pid = child.process_id();
            child_exit_status = child.try_wait().ok().flatten();
        }

        if let Err(e) = self.writer.write_all(bytes) {
            panic!(
                "write bytes: {e:?}\nchild_pid={child_pid:?}\nchild_exit_status={child_exit_status:?}\nlog_file={}\npty_tail={}",
                self.log_file.display(),
                self.tail_string(16 * 1024)
            );
        }
        if let Err(e) = self.writer.flush() {
            panic!(
                "flush bytes: {e:?}\nchild_pid={child_pid:?}\nchild_exit_status={child_exit_status:?}\nlog_file={}\npty_tail={}",
                self.log_file.display(),
                self.tail_string(16 * 1024)
            );
        }
    }

    pub fn send_ctrl_c_burst(&mut self, count: usize, gap: Duration) {
        for _ in 0..count {
            self.send_bytes(&[0x03]);
            std::thread::sleep(gap);
        }
    }

    pub fn wait_for_output_contains(&self, needle: &str, timeout: Duration) -> bool {
        wait_for_output_contains(&self.out, needle.as_bytes(), timeout)
    }

    pub fn count_output_occurrences(&self, needle: &str) -> usize {
        let needle = needle.as_bytes();
        if needle.is_empty() {
            return 0;
        }
        let h: Vec<u8> = {
            let guard = self.out.lock().expect("lock output buffer");
            guard.iter().copied().collect()
        };
        h.windows(needle.len()).filter(|w| *w == needle).count()
    }

    pub fn tail_string(&self, max_bytes: usize) -> String {
        let tail: Vec<u8> = {
            let guard = self.out.lock().expect("lock output buffer");
            guard.iter().rev().take(max_bytes).copied().collect()
        };
        let mut tail = tail;
        tail.reverse();
        String::from_utf8_lossy(&tail).into_owned()
    }
}

impl Drop for PtyHarness {
    fn drop(&mut self) {
        // Best-effort cleanup: never block in Drop.
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

fn push_capped(buf: &mut VecDeque<u8>, cap: usize, bytes: &[u8]) {
    for &b in bytes {
        if buf.len() == cap {
            buf.pop_front();
        }
        buf.push_back(b);
    }
}

fn buffer_contains(haystack: &VecDeque<u8>, needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    let h: Vec<u8> = haystack.iter().copied().collect();
    h.windows(needle.len()).any(|w| w == needle)
}

fn wait_for_output_contains(
    out: &Arc<Mutex<VecDeque<u8>>>,
    needle: &[u8],
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let has = {
            let guard = out.lock().expect("lock output buffer");
            buffer_contains(&guard, needle)
        };
        if has {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}
