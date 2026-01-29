#[derive(Debug, Clone)]
pub enum TerminalOp {
    FeedBytes(Vec<u8>),
    CursorHome,
    SetCursor {
        x: u16,
        y: u16,
    },
    ClearScreen,
    ClearScrollback,
    SetScrollback {
        rows: usize,
    },
    FillRect {
        left: u16,
        top: u16,
        right: u16,
        bottom: u16,
        ch: char,
        fg: u8,
        bg: u8,
    },
    Resize {
        rows: u16,
        cols: u16,
    },
}
