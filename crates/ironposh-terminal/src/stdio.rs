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

    /// Shared event handler for line editing and one-off checks.
    /// When `edit_line` is false, printable/paste/backspace are ignored and we only
    /// react to Enter / Ctrl+C / Ctrl+D(^Z on Windows) / Resize.
    fn process_event(
        &mut self,
        line: &mut String,
        evt: Event,
        edit_line: bool,
    ) -> io::Result<Option<ReadOutcome>> {
        match evt {
            Event::Resize(cols, rows) => {
                self.term.on_host_resize(cols, rows);
                // Keep the prompt crisp after a resize.
                self.term.render().map_err(io::Error::other)?;
                Ok(None)
            }

            // ---- ENTER ----
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code: KeyCode::Enter,
                ..
            }) => {
                self.write_all(b"\r\n")?;
                self.flush()?;
                // Return accumulated line when editing; empty string in one-off mode.
                let out = if edit_line {
                    std::mem::take(line)
                } else {
                    String::new()
                };
                Ok(Some(ReadOutcome::Line(out)))
            }

            // ---- BACKSPACE ----
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code: KeyCode::Backspace,
                ..
            }) if edit_line => {
                if !line.is_empty() {
                    line.pop();
                    self.write_all(b"\x08 \x08")?; // BS, erase, BS
                    self.flush()?;
                }
                Ok(None)
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
                Ok(Some(ReadOutcome::Interrupt))
            }

            // ---- CTRL+D / CTRL+Z → EOF ----
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code: KeyCode::Char(ch),
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL)
                && (ch == 'd' || (cfg!(windows) && ch == 'z')) =>
            {
                // Only emit EOF if the current line is empty (or we're in one-off mode).
                if !edit_line || line.is_empty() {
                    self.write_all(b"\r\n")?;
                    self.flush()?;
                    Ok(Some(ReadOutcome::Eof))
                } else {
                    Ok(None)
                }
            }

            // ---- Printable ----
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code: KeyCode::Char(c),
                modifiers,
                ..
            }) if edit_line && !modifiers.contains(KeyModifiers::CONTROL) => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                line.push(c);
                self.write_all(s.as_bytes())?;
                self.flush()?;
                Ok(None)
            }

            // ---- Paste ----
            Event::Paste(s) if edit_line => {
                line.push_str(&s);
                self.write_all(s.as_bytes())?;
                self.flush()?;
                Ok(None)
            }

            _ => Ok(None),
        }
    }

    /// Non-blocking, one-shot check: returns immediately with:
    ///   - Some(Interrupt) on ^C
    ///   - Some(Eof) on ^D (or ^Z on Windows) when no text is pending
    ///   - Some(Line("")) if Enter is pressed
    ///   - None if nothing relevant happened
    pub fn try_read_line(&mut self) -> io::Result<Option<ReadOutcome>> {
        // Zero-timeout poll: do not block.
        if !event::poll(Duration::from_millis(0))? {
            return Ok(None);
        }
        let evt = event::read()?;
        // In one-off mode we *don't* edit/echo arbitrary characters or backspace.
        let mut scratch = String::new();
        self.process_event(&mut scratch, evt, /*edit_line=*/ false)
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
                let evt = event::read()?;
                if let Some(outcome) =
                    self.process_event(&mut line, evt, /*edit_line=*/ true)?
                {
                    return Ok(outcome);
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
