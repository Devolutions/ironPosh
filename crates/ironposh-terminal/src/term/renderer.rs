use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io::{Stdout, Write};

pub trait HostRenderer {
    fn init(&mut self) -> Result<()>;
    fn present(&mut self, bytes: &[u8]) -> Result<()>;
    fn deinit(&mut self);
}

pub struct CrosstermRenderer {
    out: Stdout,
}

impl Default for CrosstermRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl CrosstermRenderer {
    pub fn new() -> Self {
        Self {
            out: std::io::stdout(),
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
