use crate::{Terminal, TerminalOp};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io::{self, Write as IoWrite};
use std::time::Duration;

#[derive(Debug)]
pub enum ReadOutcome {
    Line(String),
    Interrupt, // ^C
    Eof,       // ^D (UNIX) / ^Z (Windows)
}

/// Stdio-like wrapper that borrows a Terminal for ergonomic input/output
pub struct StdTerm<'a> {
    term: &'a mut Terminal,
    buf: Vec<u8>,
    auto_render: bool,      // paint after each flush/println
    flush_on_newline: bool, // common stdio behavior
}

impl<'a> StdTerm<'a> {
    pub(crate) fn new(term: &'a mut Terminal) -> Self {
        Self {
            term,
            buf: Vec::new(),
            auto_render: true,
            flush_on_newline: true,
        }
    }

    pub fn apply_op(&mut self, op: TerminalOp) {
        self.term.apply_op(op);
    }

    pub fn render(&mut self) -> Result<(), anyhow::Error> {
        self.term.render()
    }

    pub fn set_auto_render(&mut self, on: bool) {
        self.auto_render = on;
    }

    pub fn set_flush_on_newline(&mut self, on: bool) {
        self.flush_on_newline = on;
    }

    pub fn print<S: AsRef<[u8]>>(&mut self, s: S) -> Result<(), anyhow::Error> {
        self.write_all(s.as_ref())?;
        if self.auto_render {
            self.flush()?;
        }
        Ok(())
    }

    pub fn println<S: AsRef<[u8]>>(&mut self, s: S) -> Result<(), anyhow::Error> {
        self.write_all(s.as_ref())?;
        self.write_all(b"\r\n")?;
        if self.auto_render {
            self.flush()?;
        }
        Ok(())
    }

    /// Line-buffered input with prompt. Filters key repeats; supports paste.
    pub fn read_line(&mut self, prompt: &str) -> io::Result<ReadOutcome> {
        if !prompt.is_empty() {
            self.write_all(b"\r")?; // ensure column 0
            self.write_all(prompt.as_bytes())?;
            self.flush()?; // show prompt
        }

        let mut line = String::new();

        loop {
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Resize(cols, rows) => {
                        self.term.on_host_resize(cols, rows);
                        // Optional: repaint immediately so the prompt stays crisp after resize
                        self.term.render().map_err(io::Error::other)?;
                        continue;
                    }
                    // ---- ENTER ----
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Press,
                        code: KeyCode::Enter,
                        ..
                    }) => {
                        self.write_all(b"\r\n")?;
                        self.flush()?;
                        return Ok(ReadOutcome::Line(line));
                    }

                    // ---- BACKSPACE ----
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Press,
                        code: KeyCode::Backspace,
                        ..
                    }) => {
                        if !line.is_empty() {
                            line.pop();
                            self.write_all(b"\x08 \x08")?; // BS, erase, BS
                            self.flush()?;
                        }
                    }

                    // ---- CTRL+C → Interrupt ----
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Press,
                        code: KeyCode::Char('c'),
                        modifiers,
                        ..
                    }) if modifiers.contains(KeyModifiers::CONTROL) => {
                        // Visual ACK like real shells:
                        self.write_all(b"^C\r\n")?;
                        self.flush()?;
                        return Ok(ReadOutcome::Interrupt);
                    }

                    // ---- CTRL+D / CTRL+Z → EOF ----
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Press,
                        code: KeyCode::Char(ch),
                        modifiers,
                        ..
                    }) if modifiers.contains(KeyModifiers::CONTROL)
                        && (
                            (ch == 'd') ||                                    // UNIX ^D
                        (cfg!(windows) && ch == 'z')
                            // Windows ^Z
                        ) =>
                    {
                        if line.is_empty() {
                            self.write_all(b"\r\n")?;
                            self.flush()?;
                            return Ok(ReadOutcome::Eof);
                        }
                        // If there's text, ignore like many shells do.
                    }

                    // ---- Printable ----
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Press,
                        code: KeyCode::Char(c),
                        modifiers,
                        ..
                    }) if !modifiers.contains(KeyModifiers::CONTROL) => {
                        let mut buf = [0u8; 4];
                        let s = c.encode_utf8(&mut buf);
                        line.push(c);
                        self.write_all(s.as_bytes())?;
                        self.flush()?;
                    }

                    // ---- Paste ----
                    Event::Paste(s) => {
                        line.push_str(&s);
                        self.write_all(s.as_bytes())?;
                        self.flush()?;
                    }

                    _ => {}
                }
            }

            // optional: throttled render path
            if self.auto_render {
                self.term.render().map_err(io::Error::other)?;
            }
        }
    }
}

impl<'a> IoWrite for StdTerm<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Normalize newlines: LF -> CRLF unless already CRLF
        for &b in buf {
            if b == b'\n' && self.buf.last().copied() != Some(b'\r') {
                self.buf.push(b'\r');
            }
            self.buf.push(b);
        }

        if self.flush_on_newline && buf.ends_with(b"\n") {
            self.flush()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            let bytes = std::mem::take(&mut self.buf);
            self.term.apply_op(TerminalOp::FeedBytes(bytes));
        }
        self.term.render().map_err(io::Error::other)?;
        Ok(())
    }
}

impl<'a> Drop for StdTerm<'a> {
    fn drop(&mut self) {
        // Best-effort flush on scope exit; ignore errors in Drop.
        let _ = IoWrite::flush(self);
    }
}
