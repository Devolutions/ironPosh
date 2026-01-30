use anyhow::Result;
use ironposh_terminal::{ReadOutcome, Terminal, TerminalOp};
use std::io::Write;

fn main() -> Result<()> {
    let mut term = Terminal::new(2000)?;

    {
        let mut io = term.stdio();
        writeln!(
            io,
            "Welcome to std_echo! Type 'exit' to quit. (Ctrl+C to interrupt, Ctrl+D/Ctrl+Z to EOF)"
        )?;

        loop {
            match io.read_line("> ")? {
                ReadOutcome::Line(line) => {
                    if line.trim() == "exit" {
                        writeln!(io, "Goodbye!")?;
                        break;
                    }
                    if line.trim() == "clear" {
                        // Reset cursor to top left and clear screen using TerminalOp
                        io.apply_op(TerminalOp::CursorHome);
                        io.apply_op(TerminalOp::ClearScreen);
                        io.render()?;
                    } else {
                        writeln!(io, "You typed: {line}")?;
                    }
                }
                ReadOutcome::Interrupt => {
                    // graceful: just reprompt (like bash/zsh)
                }
                ReadOutcome::Eof => {
                    writeln!(io, "\nGoodbye!")?;
                    break;
                }
            }
        }
    }

    Ok(())
}
