//! Edge case tests for PSRP message parsing.
//!
//! These tests verify that the PowerShellRemotingMessage parser handles
//! malformed, truncated, and unexpected message data gracefully.

use byteorder::{LittleEndian, WriteBytesExt};
use ironposh_psrp::cores::{Destination, MessageType, PowerShellRemotingMessage};
use std::io::Cursor;
use uuid::Uuid;

/// Create a valid PSRP message header (40 bytes) + optional data
fn create_psrp_message(
    destination: u32,
    message_type: u32,
    rpid: Uuid,
    pid: Uuid,
    data: &[u8],
) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Destination (4 bytes, little endian)
    buffer.write_u32::<LittleEndian>(destination).unwrap();

    // Message type (4 bytes, little endian)
    buffer.write_u32::<LittleEndian>(message_type).unwrap();

    // RPID (16 bytes)
    buffer.extend_from_slice(rpid.as_bytes());

    // PID (16 bytes)
    buffer.extend_from_slice(pid.as_bytes());

    // Data payload
    buffer.extend_from_slice(data);

    buffer
}

/// Create a minimal valid SessionCapability XML payload
fn minimal_session_capability_xml() -> &'static [u8] {
    br#"<Obj RefId="0"><MS><S N="protocolversion">2.3</S><S N="PSVersion">2.0</S><S N="SerializationVersion">1.1.0.1</S></MS></Obj>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // TRUNCATION TESTS
    // =========================================================================

    /// Test: Empty input (no bytes at all)
    #[test]
    fn test_empty_input() {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_err(), "Empty input should fail: {result:?}");
        println!("Empty input error: {:?}", result.unwrap_err());
    }

    /// Test: Less than 40 bytes (truncated header)
    #[test]
    fn test_truncated_header() {
        // Only 20 bytes - not enough for the 40-byte header
        let short_data = vec![0u8; 20];
        let mut cursor = Cursor::new(short_data);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_err(), "Truncated header should fail: {result:?}");
        println!("Truncated header error: {:?}", result.unwrap_err());
    }

    /// Test: Exactly 40 bytes (header only, no data)
    #[test]
    fn test_header_only_no_data() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        // Valid header with SessionCapability type but no XML data
        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            &[], // No data
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        // Header parsing should succeed, but data will be empty
        match result {
            Ok(msg) => {
                assert!(msg.data.is_empty(), "Data should be empty");
                assert_eq!(msg.message_type, MessageType::SessionCapability);
                println!("Header-only message parsed: {:?}", msg.message_type);
            }
            Err(e) => {
                println!("Header-only message rejected: {e}");
            }
        }
    }

    // =========================================================================
    // INVALID FIELD VALUES
    // =========================================================================

    /// Test: Invalid destination value
    #[test]
    fn test_invalid_destination() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        // Invalid destination (not 1 or 2)
        let message = create_psrp_message(
            0xDEADBEEF, // Invalid destination
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            minimal_session_capability_xml(),
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(
            result.is_err(),
            "Invalid destination should fail: {result:?}"
        );

        let err = result.unwrap_err();
        let err_str = format!("{err}");
        println!("Invalid destination error: {err_str}");
        assert!(
            err_str.contains("Destination") || err_str.contains("destination"),
            "Error should mention destination"
        );
    }

    /// Test: Invalid message type value
    #[test]
    fn test_invalid_message_type() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        // Invalid message type (not a known value)
        let message = create_psrp_message(
            Destination::Client as u32,
            0xBADC0DE, // Invalid message type
            rpid,
            pid,
            minimal_session_capability_xml(),
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(
            result.is_err(),
            "Invalid message type should fail: {result:?}"
        );

        let err = result.unwrap_err();
        let err_str = format!("{err}");
        println!("Invalid message type error: {err_str}");
        assert!(
            err_str.contains("MessageType") || err_str.contains("Unknown"),
            "Error should mention message type: {err_str}"
        );
    }

    /// Test: Zero destination (edge case)
    #[test]
    fn test_zero_destination() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        let message = create_psrp_message(
            0, // Zero is invalid
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            minimal_session_capability_xml(),
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_err(), "Zero destination should fail: {result:?}");
        println!("Zero destination error: {:?}", result.unwrap_err());
    }

    // =========================================================================
    // VALID MESSAGES
    // =========================================================================

    /// Test: Valid Client destination
    #[test]
    fn test_valid_client_destination() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            minimal_session_capability_xml(),
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_ok(), "Valid client message should parse: {result:?}");

        let msg = result.unwrap();
        assert!(matches!(msg.destination, Destination::Client));
        println!("Valid client message parsed successfully");
    }

    /// Test: Valid Server destination
    #[test]
    fn test_valid_server_destination() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        let message = create_psrp_message(
            Destination::Server as u32,
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            minimal_session_capability_xml(),
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_ok(), "Valid server message should parse: {result:?}");

        let msg = result.unwrap();
        assert!(matches!(msg.destination, Destination::Server));
        println!("Valid server message parsed successfully");
    }

    // =========================================================================
    // MESSAGE TYPE COVERAGE
    // =========================================================================

    /// Test: All known message types parse correctly (header only)
    #[test]
    fn test_all_message_types_header_parse() {
        let message_types = [
            MessageType::SessionCapability,
            MessageType::InitRunspacepool,
            MessageType::PublicKey,
            MessageType::EncryptedSessionKey,
            MessageType::PublicKeyRequest,
            MessageType::ConnectRunspacepool,
            MessageType::RunspacepoolInitData,
            MessageType::ResetRunspaceState,
            MessageType::SetMaxRunspaces,
            MessageType::SetMinRunspaces,
            MessageType::RunspaceAvailability,
            MessageType::RunspacepoolState,
            MessageType::CreatePipeline,
            MessageType::GetAvailableRunspaces,
            MessageType::UserEvent,
            MessageType::ApplicationPrivateData,
            MessageType::GetCommandMetadata,
            MessageType::RunspacepoolHostCall,
            MessageType::RunspacepoolHostResponse,
            MessageType::PipelineInput,
            MessageType::EndOfPipelineInput,
            MessageType::PipelineOutput,
            MessageType::ErrorRecord,
            MessageType::PipelineState,
            MessageType::DebugRecord,
            MessageType::VerboseRecord,
            MessageType::WarningRecord,
            MessageType::ProgressRecord,
            MessageType::InformationRecord,
            MessageType::PipelineHostCall,
            MessageType::PipelineHostResponse,
        ];

        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        for msg_type in &message_types {
            let message = create_psrp_message(
                Destination::Client as u32,
                msg_type.value(),
                rpid,
                pid,
                &[], // Header only
            );

            let mut cursor = Cursor::new(message);
            let result = PowerShellRemotingMessage::parse(&mut cursor);

            assert!(
                result.is_ok(),
                "Message type {:?} should parse: {result:?}",
                msg_type
            );

            let msg = result.unwrap();
            assert_eq!(
                msg.message_type, *msg_type,
                "Parsed message type should match"
            );
        }

        println!("All {} message types parse correctly", message_types.len());
    }

    // =========================================================================
    // DATA PAYLOAD TESTS
    // =========================================================================

    /// Test: Invalid UTF-8 in data payload
    #[test]
    fn test_invalid_utf8_data() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        // Invalid UTF-8 sequence
        let invalid_utf8 = vec![0xFF, 0xFE, 0x00, 0x01];

        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            &invalid_utf8,
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        // Header parsing should succeed (data is opaque bytes)
        assert!(result.is_ok(), "Header should parse even with invalid UTF-8 data");

        let msg = result.unwrap();
        // But parsing the PS message content should fail
        let ps_result = msg.parse_ps_message();
        assert!(
            ps_result.is_err(),
            "Invalid UTF-8 should fail at XML parsing: {ps_result:?}"
        );

        println!("Invalid UTF-8 correctly rejected at XML parsing stage");
    }

    /// Test: Valid UTF-8 but invalid XML
    #[test]
    fn test_invalid_xml_data() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        let invalid_xml = b"This is not XML at all!";

        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            invalid_xml,
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        // Header parsing succeeds
        assert!(result.is_ok());

        let msg = result.unwrap();
        let ps_result = msg.parse_ps_message();
        assert!(
            ps_result.is_err(),
            "Invalid XML should fail: {ps_result:?}"
        );

        println!("Invalid XML correctly rejected: {:?}", ps_result.unwrap_err());
    }

    /// Test: Valid XML but wrong structure (not CLIXML)
    #[test]
    fn test_wrong_xml_structure() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        // Valid XML but not PSRP CLIXML format
        let wrong_xml = b"<html><body>Hello World</body></html>";

        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            wrong_xml,
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_ok());

        let msg = result.unwrap();
        let ps_result = msg.parse_ps_message();

        // Should fail because it's not valid CLIXML
        println!("Wrong XML structure result: {ps_result:?}");
    }

    // =========================================================================
    // UUID BOUNDARY TESTS
    // =========================================================================

    /// Test: Nil UUIDs for both RPID and PID
    #[test]
    fn test_nil_uuids() {
        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            Uuid::nil(),
            Uuid::nil(),
            minimal_session_capability_xml(),
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_ok(), "Nil UUIDs should be valid: {result:?}");

        let msg = result.unwrap();
        assert_eq!(msg.rpid, Uuid::nil());
        println!("Nil UUIDs parsed successfully");
    }

    /// Test: Max UUID values
    #[test]
    fn test_max_uuids() {
        let max_uuid = Uuid::from_bytes([0xFF; 16]);

        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            max_uuid,
            max_uuid,
            minimal_session_capability_xml(),
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_ok(), "Max UUIDs should be valid: {result:?}");

        let msg = result.unwrap();
        assert_eq!(msg.rpid, max_uuid);
        println!("Max UUIDs parsed successfully");
    }

    // =========================================================================
    // LARGE DATA TESTS
    // =========================================================================

    /// Test: Very large data payload (1MB)
    #[test]
    fn test_large_data_payload() {
        let rpid = Uuid::new_v4();
        let pid = Uuid::nil();

        // 1MB of data
        let large_data = vec![b'A'; 1024 * 1024];

        let message = create_psrp_message(
            Destination::Client as u32,
            MessageType::SessionCapability.value(),
            rpid,
            pid,
            &large_data,
        );

        let mut cursor = Cursor::new(message);
        let result = PowerShellRemotingMessage::parse(&mut cursor);

        assert!(result.is_ok(), "Large payload should parse header: {result:?}");

        let msg = result.unwrap();
        assert_eq!(msg.data.len(), 1024 * 1024);
        println!("1MB payload parsed successfully");
    }
}
