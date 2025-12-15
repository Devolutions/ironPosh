use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ironposh_terminal::{Terminal, TerminalOp};
use std::time::Duration;

fn main() -> Result<()> {
    let mut term = Terminal::new(2000)?;
    let mut line = String::new();

    // initial frame
    term.apply_ops(vec![
        TerminalOp::ClearScreen,
        TerminalOp::FeedBytes(b"> ".to_vec()),
    ]);
    term.render()?;

    loop {
        // non-blocking poll for keys
        if event::poll(Duration::from_millis(25))? {
            let ev = event::read()?;

            // Only handle actual key presses; ignore Repeat/Release
            if let Event::Key(KeyEvent { kind, .. }) = ev
                && kind != KeyEventKind::Press
            {
                continue;
            }

            match ev {
                Event::Resize(cols, rows) => {
                    term.on_host_resize(cols, rows);
                    term.render()?;
                }

                // Ctrl+C exits cleanly
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers,
                    ..
                }) if modifiers.contains(KeyModifiers::CONTROL) => {
                    break;
                }

                // Enter: echo the full line, reset buffer, re-prompt
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    ..
                }) => {
                    let mut output = b"\r\n".to_vec();
                    output.extend_from_slice(line.as_bytes()); // echo the whole typed line
                    output.extend_from_slice(b"\r\n> "); // new prompt

                    term.apply_ops(vec![TerminalOp::FeedBytes(output)]);
                    line.clear();
                }

                // Printable characters
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers,
                    ..
                }) => {
                    // handle shift etc. automatically via c
                    if !modifiers.contains(KeyModifiers::CONTROL) {
                        line.push(c);
                        let mut buf = [0u8; 4];
                        term.apply_ops(vec![TerminalOp::FeedBytes(
                            c.encode_utf8(&mut buf).as_bytes().to_vec(),
                        )]);
                    }
                }

                // Backspace
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                }) => {
                    if !line.is_empty() {
                        line.pop();
                        // Move cursor left, write space, move cursor left again
                        term.apply_ops(vec![TerminalOp::FeedBytes(b"\x08 \x08".to_vec())]);
                    }
                }

                // Optional: handle paste (if supported by your crossterm version)
                #[allow(unreachable_patterns)]
                Event::Paste(s) => {
                    line.push_str(&s);
                    term.apply_ops(vec![TerminalOp::FeedBytes(s.into_bytes())]);
                }

                _ => {}
            }
            term.render()?; // present changes
        }
    }

    Ok(())
}
