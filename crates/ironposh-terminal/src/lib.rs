use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::{debug, info, instrument, trace};

pub mod input;
pub mod stdio;
pub mod term;

pub use stdio::{ReadOutcome, StdTerm};
pub use term::{CrosstermRenderer, GuestTerm, HostRenderer, TerminalOp};

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
        debug!(?op, "Applying terminal operation");
        self.guest.apply(op);
    }

    /// Update guest size when the host reports a resize event.
    pub fn on_host_resize(&mut self, cols: u16, rows: u16) {
        self.apply_op(TerminalOp::Resize { rows, cols });
    }

    /// Render the terminal if dirty
    pub fn render(&mut self) -> Result<()> {
        trace!(dirty = self.guest.is_dirty(), "Render called");

        // Simple throttle to avoid spamming the host terminal
        if self.last_render.elapsed() < Duration::from_millis(8) {
            trace!("Skipping render due to throttle");
            return Ok(());
        }

        if let Some(bytes) = self.guest.take_render_bytes() {
            self.last_render = Instant::now();
            trace!(bytes_len = bytes.len(), "Presenting bytes to renderer");
            self.renderer.present(&bytes)?;
        }

        Ok(())
    }

    /// Get the current terminal size
    pub fn size(&mut self) -> Result<(u16, u16)> {
        self.renderer.host_size()
    }

    /// Returns the current guest screen size as (rows, cols).
    pub fn guest_screen_size(&self) -> (u16, u16) {
        self.guest.screen_size()
    }

    /// Returns the current guest cursor position as (row, col).
    pub fn guest_cursor_position(&self) -> (u16, u16) {
        self.guest.cursor_position()
    }

    /// Returns a snapshot of the cell at (row, col) from the guest screen.
    pub fn guest_cell(&self, row: u16, col: u16) -> Option<vt100::Cell> {
        self.guest.cell(row, col)
    }

    /// Borrow a stdio-like handle. Scope it to release the &mut borrow when done.
    pub fn stdio(&mut self) -> StdTerm<'_> {
        StdTerm::new(self)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.renderer.deinit();
    }
}
