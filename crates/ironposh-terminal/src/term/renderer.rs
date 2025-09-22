use std::io::{Stdout, Write};
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use anyhow::Result;

pub trait HostRenderer {
    fn init(&mut self) -> Result<()>;
    fn present(&mut self, bytes: &[u8]) -> Result<()>;
    fn deinit(&mut self);
}

pub struct CrosstermRenderer {
    out: Stdout,
    last_rows: u16,
    last_cols: u16,
}

impl CrosstermRenderer {
    pub fn new() -> Self {
        Self {
            out: std::io::stdout(),
            last_rows: 0,
            last_cols: 0,
        }
    }

    pub fn host_size(&mut self) -> Result<(u16, u16)> {
        Ok(crossterm::terminal::size()?)
    }
}

impl HostRenderer for CrosstermRenderer {
    fn init(&mut self) -> Result<()> {
        enable_raw_mode()?;
        self.out.execute(EnterAlternateScreen)?;
        Ok(())
    }

    fn present(&mut self, bytes: &[u8]) -> Result<()> {
        self.out.write_all(bytes)?;
        self.out.flush()?;
        Ok(())
    }

    fn deinit(&mut self) {
        let _ = self.out.execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}