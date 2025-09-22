use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub enum KeyAction {
    SendBytes(Vec<u8>),
    ExitProgram,
    Ignore,
}

pub fn key_to_action(key: KeyEvent) -> KeyAction {
    match key.code {
        // Ctrl+C - send interrupt or exit
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::ExitProgram
        }

        // Regular printable characters
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Handle Ctrl+key combinations
                match c {
                    'a' => KeyAction::SendBytes(vec![0x01]), // Ctrl+A
                    'b' => KeyAction::SendBytes(vec![0x02]), // Ctrl+B
                    'd' => KeyAction::SendBytes(vec![0x04]), // Ctrl+D
                    'e' => KeyAction::SendBytes(vec![0x05]), // Ctrl+E
                    'f' => KeyAction::SendBytes(vec![0x06]), // Ctrl+F
                    'g' => KeyAction::SendBytes(vec![0x07]), // Ctrl+G
                    'h' => KeyAction::SendBytes(vec![0x08]), // Ctrl+H (backspace)
                    'k' => KeyAction::SendBytes(vec![0x0B]), // Ctrl+K
                    'l' => KeyAction::SendBytes(vec![0x0C]), // Ctrl+L
                    'n' => KeyAction::SendBytes(vec![0x0E]), // Ctrl+N
                    'p' => KeyAction::SendBytes(vec![0x10]), // Ctrl+P
                    'r' => KeyAction::SendBytes(vec![0x12]), // Ctrl+R
                    't' => KeyAction::SendBytes(vec![0x14]), // Ctrl+T
                    'u' => KeyAction::SendBytes(vec![0x15]), // Ctrl+U
                    'w' => KeyAction::SendBytes(vec![0x17]), // Ctrl+W
                    'y' => KeyAction::SendBytes(vec![0x19]), // Ctrl+Y
                    'z' => KeyAction::SendBytes(vec![0x1A]), // Ctrl+Z
                    _ => KeyAction::Ignore,
                }
            } else {
                // Regular character - encode as UTF-8
                KeyAction::SendBytes(c.to_string().into_bytes())
            }
        }

        // Special keys
        KeyCode::Enter => KeyAction::SendBytes(b"\r\n".to_vec()),
        KeyCode::Tab => KeyAction::SendBytes(b"\t".to_vec()),
        KeyCode::Backspace => KeyAction::SendBytes(b"\x7F".to_vec()), // DEL
        KeyCode::Delete => KeyAction::SendBytes(b"\x1b[3~".to_vec()),
        KeyCode::Esc => KeyAction::SendBytes(b"\x1b".to_vec()),

        // Arrow keys
        KeyCode::Up => KeyAction::SendBytes(b"\x1b[A".to_vec()),
        KeyCode::Down => KeyAction::SendBytes(b"\x1b[B".to_vec()),
        KeyCode::Right => KeyAction::SendBytes(b"\x1b[C".to_vec()),
        KeyCode::Left => KeyAction::SendBytes(b"\x1b[D".to_vec()),

        // Home/End
        KeyCode::Home => KeyAction::SendBytes(b"\x1b[H".to_vec()),
        KeyCode::End => KeyAction::SendBytes(b"\x1b[F".to_vec()),

        // Page Up/Down
        KeyCode::PageUp => KeyAction::SendBytes(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => KeyAction::SendBytes(b"\x1b[6~".to_vec()),

        // Function keys
        KeyCode::F(n) => match n {
            1 => KeyAction::SendBytes(b"\x1b[11~".to_vec()),
            2 => KeyAction::SendBytes(b"\x1b[12~".to_vec()),
            3 => KeyAction::SendBytes(b"\x1b[13~".to_vec()),
            4 => KeyAction::SendBytes(b"\x1b[14~".to_vec()),
            5 => KeyAction::SendBytes(b"\x1b[15~".to_vec()),
            6 => KeyAction::SendBytes(b"\x1b[17~".to_vec()),
            7 => KeyAction::SendBytes(b"\x1b[18~".to_vec()),
            8 => KeyAction::SendBytes(b"\x1b[19~".to_vec()),
            9 => KeyAction::SendBytes(b"\x1b[20~".to_vec()),
            10 => KeyAction::SendBytes(b"\x1b[21~".to_vec()),
            11 => KeyAction::SendBytes(b"\x1b[23~".to_vec()),
            12 => KeyAction::SendBytes(b"\x1b[24~".to_vec()),
            _ => KeyAction::Ignore,
        },

        // Insert
        KeyCode::Insert => KeyAction::SendBytes(b"\x1b[2~".to_vec()),

        // Other keys we don't handle
        _ => KeyAction::Ignore,
    }
}

/// Convert key bytes back to VT sequences that can be sent to the remote PowerShell session
pub fn key_bytes_to_vt_input(bytes: &[u8]) -> Vec<u8> {
    // For now, just pass through the bytes as-is
    // In the future, this could do more sophisticated translation
    bytes.to_vec()
}