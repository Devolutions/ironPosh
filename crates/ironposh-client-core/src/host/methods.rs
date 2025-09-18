use uuid::Uuid;
use ironposh_psrp::PsValue;
use std::collections::HashMap;

// Strongly-typed data structures per MS-PSRP spec 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinates { pub x: i32, pub y: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size { pub width: i32, pub height: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rectangle { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

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

// TODO: The rest of this file contains old host call system enums that are being replaced
// by the new typesafe system in mod.rs. These can be removed once the transition is complete.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{RemoteHostMethodId, should_send_host_response};

    #[test]
    fn test_spec_compliant_method_ids() {
        // Test that our method IDs match the MS-PSRP spec exactly
        assert_eq!(RemoteHostMethodId::GetName as i32, 1);
        assert_eq!(RemoteHostMethodId::GetCursorPosition as i32, 31); 
        assert_eq!(RemoteHostMethodId::SetCursorPosition as i32, 32);
        assert_eq!(RemoteHostMethodId::GetForegroundColor as i32, 27);
        assert_eq!(RemoteHostMethodId::SetForegroundColor as i32, 28);
        assert_eq!(RemoteHostMethodId::PromptForChoiceMultipleSelection as i32, 56);
    }

    #[test]
    fn test_response_gating() {
        // Test that only methods with return values require responses
        assert!(should_send_host_response(RemoteHostMethodId::GetName));
        assert!(should_send_host_response(RemoteHostMethodId::ReadLine));
        assert!(should_send_host_response(RemoteHostMethodId::GetCursorPosition));
        
        // Test that void methods do NOT require responses
        assert!(!should_send_host_response(RemoteHostMethodId::SetShouldExit));
        assert!(!should_send_host_response(RemoteHostMethodId::SetCursorPosition));
        assert!(!should_send_host_response(RemoteHostMethodId::WriteProgress));
    }
}
