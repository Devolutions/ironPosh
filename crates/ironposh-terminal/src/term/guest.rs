use super::TerminalOp;

pub struct FillRectParams {
    pub l: u16,
    pub t: u16,
    pub r: u16,
    pub b: u16,
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
        use TerminalOp::*;
        match op {
            FeedBytes(bytes) => self.feed(&bytes),
            CursorHome => self.feed(b"\x1b[H"),
            SetCursor { x, y } => self.feed(format!("\x1b[{};{}H", y + 1, x + 1).as_bytes()),
            ClearScreen => self.feed(b"\x1b[2J\x1b[H"),
            ClearScrollback => self.feed(b"\x1b[3J\x1b[2J\x1b[H"),
            Resize { rows, cols } => {
                self.parser.screen_mut().set_size(rows, cols);
                self.prev = None;
                self.dirty = true;
            }
            FillRect {
                left,
                top,
                right,
                bottom,
                ch,
                fg,
                bg,
            } => self.fill_rect(FillRectParams {
                l: left,
                t: top,
                r: right,
                b: bottom,
                ch,
                fg,
                bg,
            }),
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.dirty = true;
    }

    fn fill_rect(&mut self, rect: FillRectParams) {
        let FillRectParams {
            l,
            t,
            r,
            b,
            ch,
            fg,
            bg,
        } = rect;
        if ch == ' ' && l == 0 && t == 0 {
            self.feed(b"\x1b[2J\x1b[H");
            return;
        }
        let fg = idx_to_sgr_fg(fg);
        let bg = idx_to_sgr_bg(bg);
        self.feed(b"\x1b7"); // save cursor
        let width = (r - l + 1) as usize;
        let run = ch.to_string().repeat(width);
        let sgr = format!("\x1b[{fg};{bg}m");
        for y in t..=b {
            let seq = format!("\x1b[{};{}H{}{}", y + 1, l + 1, sgr, run);
            self.feed(seq.as_bytes());
        }
        self.feed(b"\x1b[0m\x1b8"); // reset attrs + restore cursor
    }

    /// Produce bytes to render: full on first frame, diffs after, and keep host cursor in sync.
    pub fn take_render_bytes(&mut self) -> Option<Vec<u8>> {
        if !self.dirty {
            return None;
        }
        let screen = self.parser.screen().clone();
        let mut bytes = if let Some(prev) = &self.prev {
            screen.state_diff(prev)
        } else {
            screen.state_formatted()
        };
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
