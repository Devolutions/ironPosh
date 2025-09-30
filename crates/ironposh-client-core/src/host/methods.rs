use ironposh_psrp::PsValue;

// Strongly-typed data structures per MS-PSRP spec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinates {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rectangle {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferCell {
    pub character: char,
    pub foreground: i32, // Color enum underlying int
    pub background: i32, // Color enum underlying int
    pub flags: i32,      // BufferCellType, underlying int
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    pub virtual_key_code: i32,
    pub character: char,
    pub control_key_state: i32,
    pub key_down: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressRecord {
    pub activity: String,
    pub status_description: String,
    pub current_operation: String,
    pub activity_id: i32,
    pub parent_activity_id: i32,
    pub percent_complete: i32,
    pub seconds_remaining: i32,
    pub record_type: i32, // ProgressRecordType
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDescription {
    pub name: String,
    pub label: String,
    pub help_message: String,
    pub is_mandatory: bool,
    pub parameter_type: String,
    pub default_value: Option<PsValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceDescription {
    pub label: String,
    pub help_message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PSCredential {
    pub user_name: String,
    pub password: Vec<u8>, // SecureString as bytes
}
