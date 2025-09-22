use std::time::{Duration, Instant};
use anyhow::Result;
use crossterm::terminal;
use tracing::{debug, info, instrument};

pub mod term;
pub mod input;

pub use term::{TerminalOp, GuestTerm, HostRenderer, CrosstermRenderer};

/// Clean terminal pipeline with separated concerns
pub struct Terminal {
    guest: GuestTerm,
    renderer: CrosstermRenderer,
    last_render: Instant,
}

impl Terminal {
    /// Create a terminal bound to the current host size.
    #[instrument]
    pub fn new(scrollback: usize) -> Result<Self> {
        info!("Initializing terminal emulator");

        // Get host size for guest terminal
        let (cols, rows) = crossterm::terminal::size()?;
        info!(
            "Terminal initialized with size: {}x{}, scrollback: {}",
            cols, rows, scrollback
        );

        let guest = GuestTerm::new(rows, cols, scrollback);
        let mut renderer = CrosstermRenderer::new();
        renderer.init()?;

        Ok(Self {
            guest,
            renderer,
            last_render: Instant::now(),
        })
    }

    /// Apply terminal operations to the guest
    pub fn apply_ops(&mut self, ops: Vec<TerminalOp>) {
        for op in ops {
            self.guest.apply(op);
        }
    }

    /// Apply a single terminal operation
    pub fn apply_op(&mut self, op: TerminalOp) {
        self.guest.apply(op);
    }

    /// Non-consuming resize: compare current host size and update screen if changed.
    pub fn sync_host_size_if_changed(&mut self) -> Result<bool> {
        let (cols, rows) = self.renderer.host_size()?;
        debug!(current_cols = cols, current_rows = rows, "Checking terminal size");

        // Apply resize operation if size changed
        self.guest.apply(TerminalOp::Resize { rows, cols });
        Ok(true) // For now, always assume change (guest handles dirty checking)
    }

    /// Render the terminal if dirty
    pub fn render(&mut self) -> Result<()> {
        debug!(dirty = self.guest.is_dirty(), "Render called");

        // Simple throttle to avoid spamming the host terminal
        if self.last_render.elapsed() < Duration::from_millis(8) {
            debug!("Skipping render due to throttle");
            return Ok(());
        }

        if let Some(bytes) = self.guest.take_render_bytes() {
            self.last_render = Instant::now();
            debug!(bytes_len = bytes.len(), "Presenting bytes to renderer");
            self.renderer.present(&bytes)?;
            info!(bytes_written = bytes.len(), "Terminal render completed");
        }

        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.renderer.deinit();
    }
}