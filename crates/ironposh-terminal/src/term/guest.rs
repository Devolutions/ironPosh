use super::TerminalOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillArea {
    /// Fill the entire screen
    FullScreen,
    /// Fill a specific rectangular area
    Rectangle { l: u16, t: u16, r: u16, b: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillRectParams {
    pub area: FillArea,
    pub ch: char,
    pub fg: u8,
    pub bg: u8,
}

pub struct GuestTerm {
    parser: vt100::Parser,
    prev: Option<vt100::Screen>,
    dirty: bool,
}

impl GuestTerm {
    pub fn new(rows: u16, cols: u16, scrollback: usize) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, scrollback),
            prev: None,
            dirty: true,
        }
    }

    pub fn apply(&mut self, op: TerminalOp) {
        match op {
            TerminalOp::FeedBytes(bytes) => self.feed(&bytes),
            TerminalOp::CursorHome => self.feed(b"\x1b[H"),
            TerminalOp::SetCursor { x, y } => {
                self.feed(format!("\x1b[{};{}H", y + 1, x + 1).as_bytes());
            }
            TerminalOp::ClearScreen => self.feed(b"\x1b[2J\x1b[H"),
            TerminalOp::ClearScrollback => self.feed(b"\x1b[3J\x1b[2J\x1b[H"),
            TerminalOp::Resize { rows, cols } => {
                self.parser.screen_mut().set_size(rows, cols);
                self.prev = None;
                self.dirty = true;
            }
            TerminalOp::FillRect {
                left,
                top,
                right,
                bottom,
                ch,
                fg,
                bg,
            } => {
                let area = FillArea::Rectangle {
                    l: left,
                    t: top,
                    r: right,
                    b: bottom,
                };
                self.fill_rect(FillRectParams { area, ch, fg, bg });
            }
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.dirty = true;
    }

    fn fill_rect(&mut self, rect: FillRectParams) {
        let FillRectParams { area, ch, fg, bg } = rect;

        match area {
            FillArea::FullScreen => {
                // Clear the entire screen
                self.feed(b"\x1b[2J\x1b[H");
            }
            FillArea::Rectangle { l, t, r, b } => {
                // Handle specific rectangle fill
                if ch == ' ' && l == 0 && t == 0 {
                    self.feed(b"\x1b[2J\x1b[H");
                    return;
                }

                let fg = idx_to_sgr_fg(fg);
                let bg = idx_to_sgr_bg(bg);
                self.feed(b"\x1b7"); // save cursor

                // Use wider arithmetic to prevent overflow
                let width = (r as u32 - l as u32 + 1) as usize;
                let run = ch.to_string().repeat(width);
                let sgr = format!("\x1b[{fg};{bg}m");

                for y in t..=b {
                    // Use u32 arithmetic to prevent overflow when adding 1
                    let seq = format!("\x1b[{};{}H{}{}", (y as u32) + 1, (l as u32) + 1, sgr, run);
                    self.feed(seq.as_bytes());
                }
                self.feed(b"\x1b[0m\x1b8"); // reset attrs + restore cursor
            }
        }
    }

    /// Produce bytes to render: full on first frame, diffs after, and keep host cursor in sync.
    pub fn take_render_bytes(&mut self) -> Option<Vec<u8>> {
        if !self.dirty {
            return None;
        }
        let screen = self.parser.screen().clone();
        let mut bytes = self
            .prev
            .as_ref()
            .map_or_else(|| screen.state_formatted(), |prev| screen.state_diff(prev));
        bytes.extend_from_slice(&screen.cursor_state_formatted());
        self.prev = Some(screen);
        self.dirty = false;
        Some(bytes)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

// trivial color index maps
fn idx_to_sgr_fg(i: u8) -> &'static str {
    match i {
        0 => "30",
        1 => "34",
        2 => "32",
        3 => "36",
        4 => "31",
        5 => "35",
        6 => "33",
        7 => "37",
        8 => "90",
        9 => "94",
        10 => "92",
        11 => "96",
        12 => "91",
        13 => "95",
        14 => "93",
        15 => "97",
        _ => "39",
    }
}

fn idx_to_sgr_bg(i: u8) -> &'static str {
    match i {
        0 => "40",
        1 => "44",
        2 => "42",
        3 => "46",
        4 => "41",
        5 => "45",
        6 => "43",
        7 => "47",
        8 => "100",
        9 => "104",
        10 => "102",
        11 => "106",
        12 => "101",
        13 => "105",
        14 => "103",
        15 => "107",
        _ => "49",
    }
}
