//! Edge case tests for the PSRP fragmentation/defragmentation layer.
//!
//! These tests verify that the defragmenter handles malformed, truncated,
//! and unexpected fragment data gracefully.

use byteorder::{BigEndian, WriteBytesExt};
use ironposh_psrp::fragmentation::{DefragmentResult, Defragmenter};

/// Create a minimal valid fragment header + data
fn create_fragment(
    object_id: u64,
    fragment_id: u64,
    start: bool,
    end: bool,
    data: &[u8],
) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Object ID (8 bytes, big endian)
    buffer.write_u64::<BigEndian>(object_id).unwrap();

    // Fragment ID (8 bytes, big endian)
    buffer.write_u64::<BigEndian>(fragment_id).unwrap();

    // Start/End flags (1 byte)
    let mut flags = 0u8;
    if start {
        flags |= 0x01;
    }
    if end {
        flags |= 0x02;
    }
    buffer.push(flags);

    // Data length (4 bytes, big endian)
    buffer.write_u32::<BigEndian>(data.len() as u32).unwrap();

    // Data payload
    buffer.extend_from_slice(data);

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // TRUNCATION TESTS
    // =========================================================================

    /// Test: Fragment data completely empty
    #[test]
    fn test_empty_input() {
        let mut defrag = Defragmenter::new();
        let result = defrag.defragment(&[]);

        // Empty input should return Incomplete (nothing to process)
        match result {
            Ok(DefragmentResult::Incomplete) => {
                println!("Empty input correctly returns Incomplete");
            }
            Ok(DefragmentResult::Complete(_)) => {
                panic!("Empty input should not produce complete messages");
            }
            Err(e) => {
                // Also acceptable - depends on implementation
                println!("Empty input returned error (acceptable): {e}");
            }
        }
    }

    /// Test: Fragment header truncated (less than 21 bytes minimum)
    #[test]
    fn test_truncated_header() {
        let mut defrag = Defragmenter::new();

        // Only 10 bytes - not enough for header
        let short_data = vec![0u8; 10];
        let result = defrag.defragment(&short_data);

        assert!(
            result.is_err(),
            "Truncated header should fail, got: {result:?}"
        );

        let err = result.unwrap_err();
        let err_str = format!("{err}");
        println!("Truncated header error: {err_str}");
        assert!(
            err_str.contains("too short")
                || err_str.contains("truncated")
                || err_str.contains("21"),
            "Error should mention size requirement"
        );
    }

    /// Test: Header claims more data than available
    #[test]
    fn test_truncated_data_payload() {
        let mut defrag = Defragmenter::new();

        // Create header that claims 1000 bytes of data, but only provide 10
        let mut buffer = Vec::new();
        buffer.write_u64::<BigEndian>(1).unwrap(); // object_id
        buffer.write_u64::<BigEndian>(0).unwrap(); // fragment_id
        buffer.push(0x03); // flags: start + end
        buffer.write_u32::<BigEndian>(1000).unwrap(); // claims 1000 bytes
        buffer.extend_from_slice(&[0u8; 10]); // but only 10 bytes present

        let result = defrag.defragment(&buffer);

        assert!(
            result.is_err(),
            "Truncated payload should fail, got: {result:?}"
        );

        let err = result.unwrap_err();
        println!("Truncated payload error: {err}");
    }

    /// Test: Exactly 21 bytes (header only, 0-length data) with start+end flags
    #[test]
    fn test_zero_length_data_fragment() {
        let mut defrag = Defragmenter::new();

        // Valid header with 0 bytes of data
        let fragment = create_fragment(1, 0, true, true, &[]);

        let result = defrag.defragment(&fragment);

        // Zero-length fragment is syntactically valid, but may fail at message parsing
        match result {
            Ok(DefragmentResult::Complete(_)) => {
                println!("Zero-length fragment accepted (lenient)");
            }
            Ok(DefragmentResult::Incomplete) => {
                println!("Zero-length fragment returned incomplete");
            }
            Err(e) => {
                // Expected - empty data can't be a valid PSRP message
                println!("Zero-length fragment correctly rejected: {e}");
            }
        }
    }

    // =========================================================================
    // OUT OF ORDER / DUPLICATE TESTS
    // =========================================================================

    /// Test: Fragments arrive in reverse order (2, 1, 0) instead of (0, 1, 2)
    #[test]
    fn test_out_of_order_fragments() {
        let mut defrag = Defragmenter::new();

        // Create 3 fragments for object_id=1
        // Fragment 0: start, data="AAA"
        // Fragment 1: middle, data="BBB"
        // Fragment 2: end, data="CCC"

        let frag0 = create_fragment(1, 0, true, false, b"AAA");
        let frag1 = create_fragment(1, 1, false, false, b"BBB");
        let frag2 = create_fragment(1, 2, false, true, b"CCC");

        // Send in reverse order
        let result2 = defrag.defragment(&frag2);
        println!("After frag2 (end): {result2:?}");
        assert!(
            matches!(result2, Ok(DefragmentResult::Incomplete) | Err(_)),
            "End fragment alone should not complete"
        );

        let result1 = defrag.defragment(&frag1);
        println!("After frag1 (middle): {result1:?}");

        let result0 = defrag.defragment(&frag0);
        println!("After frag0 (start): {result0:?}");

        // Depending on implementation, either:
        // - It reassembles correctly (sorts by fragment_id)
        // - It fails/returns incomplete (strict ordering required)
        // Both are acceptable behaviors - we're testing it doesn't panic

        println!(
            "Out-of-order test completed. Pending buffers: {}",
            defrag.pending_count()
        );
    }

    /// Test: Same fragment arrives twice
    #[test]
    fn test_duplicate_fragment() {
        let mut defrag = Defragmenter::new();

        // Create a multi-fragment message
        let frag0 = create_fragment(1, 0, true, false, b"DATA1");
        let frag1 = create_fragment(1, 1, false, true, b"DATA2");

        // Send frag0 twice
        let _ = defrag.defragment(&frag0);
        let _ = defrag.defragment(&frag0); // duplicate!

        let result = defrag.defragment(&frag1);
        println!("Duplicate fragment handling: {result:?}");

        // Should either complete or handle the duplicate gracefully
        // Main test is that we don't panic or corrupt state
    }

    // =========================================================================
    // MISSING FRAGMENT TESTS
    // =========================================================================

    /// Test: Start and end fragments present, but middle is missing
    #[test]
    fn test_missing_middle_fragment() {
        let mut defrag = Defragmenter::new();

        // Fragment 0: start
        let frag0 = create_fragment(1, 0, true, false, b"START");
        // Fragment 1: (missing!)
        // Fragment 2: end
        let frag2 = create_fragment(1, 2, false, true, b"END");

        let _ = defrag.defragment(&frag0);
        let result = defrag.defragment(&frag2);

        // Should remain incomplete or fail - can't reassemble without middle
        println!("Missing middle fragment result: {result:?}");
        println!("Pending buffers: {}", defrag.pending_count());
    }

    /// Test: Only end fragment, no start
    #[test]
    fn test_missing_start_fragment() {
        let mut defrag = Defragmenter::new();

        // Only send end fragment
        let frag_end = create_fragment(1, 5, false, true, b"END_ONLY");

        let result = defrag.defragment(&frag_end);

        // Should not complete - we never got the start
        match result {
            Ok(DefragmentResult::Incomplete) => {
                println!("Missing start correctly returns Incomplete");
            }
            Ok(DefragmentResult::Complete(_)) => {
                // This would be surprising but not necessarily wrong
                println!("Warning: Completed without start fragment");
            }
            Err(e) => {
                println!("Missing start returned error: {e}");
            }
        }
    }

    // =========================================================================
    // INTERLEAVED MESSAGES TESTS
    // =========================================================================

    /// Test: Two different object_ids have fragments interleaved
    #[test]
    fn test_interleaved_objects() {
        let mut defrag = Defragmenter::new();

        // Object 1: fragments 0, 1
        let obj1_frag0 = create_fragment(1, 0, true, false, b"OBJ1_START");
        let obj1_frag1 = create_fragment(1, 1, false, true, b"OBJ1_END");

        // Object 2: fragments 0, 1
        let obj2_frag0 = create_fragment(2, 0, true, false, b"OBJ2_START");
        let obj2_frag1 = create_fragment(2, 1, false, true, b"OBJ2_END");

        // Interleave: obj1_f0, obj2_f0, obj1_f1, obj2_f1
        let _ = defrag.defragment(&obj1_frag0);
        assert_eq!(
            defrag.pending_count(),
            1,
            "Should have 1 pending after obj1_f0"
        );

        let _ = defrag.defragment(&obj2_frag0);
        assert_eq!(
            defrag.pending_count(),
            2,
            "Should have 2 pending after obj2_f0"
        );

        let result1 = defrag.defragment(&obj1_frag1);
        println!(
            "After obj1 complete: {result1:?}, pending: {}",
            defrag.pending_count()
        );

        let result2 = defrag.defragment(&obj2_frag1);
        println!(
            "After obj2 complete: {result2:?}, pending: {}",
            defrag.pending_count()
        );

        // Both should eventually complete, pending should be 0
        // (or close to 0 if messages failed to parse)
    }

    // =========================================================================
    // INVALID FLAG COMBINATIONS
    // =========================================================================

    /// Test: Fragment with neither start nor end flag (orphan middle)
    #[test]
    fn test_orphan_middle_fragment() {
        let mut defrag = Defragmenter::new();

        // Middle fragment with no start ever sent
        let orphan = create_fragment(99, 5, false, false, b"ORPHAN");

        let result = defrag.defragment(&orphan);

        // Should buffer it (waiting for more) or reject it
        println!("Orphan middle fragment: {result:?}");
        println!("Pending: {}", defrag.pending_count());
    }

    /// Test: Fragment with both start and end (single-fragment message) but invalid content
    #[test]
    fn test_single_fragment_invalid_content() {
        let mut defrag = Defragmenter::new();

        // Valid fragment structure, but garbage data that's not a valid PSRP message
        let garbage = create_fragment(1, 0, true, true, b"NOT_A_VALID_PSRP_MESSAGE");

        let result = defrag.defragment(&garbage);

        // Should fail at message parsing stage
        assert!(
            result.is_err(),
            "Invalid PSRP content should fail parsing, got: {result:?}"
        );

        println!(
            "Invalid content correctly rejected: {:?}",
            result.unwrap_err()
        );
    }

    // =========================================================================
    // BOUNDARY CONDITIONS
    // =========================================================================

    /// Test: Very large fragment_id values (near u64::MAX)
    #[test]
    fn test_large_fragment_ids() {
        let mut defrag = Defragmenter::new();

        let large_id = u64::MAX - 1;
        let fragment = create_fragment(large_id, large_id, true, true, b"data");

        let result = defrag.defragment(&fragment);

        // Should handle large IDs without overflow issues
        println!("Large fragment ID result: {result:?}");
    }

    /// Test: Multiple complete single-fragment messages in one packet
    #[test]
    fn test_multiple_messages_one_packet() {
        let mut defrag = Defragmenter::new();

        // Two complete (start+end) fragments concatenated
        // Note: These will fail at PSRP parsing, but fragment layer should handle them
        let frag1 = create_fragment(1, 0, true, true, b"MSG1");
        let frag2 = create_fragment(2, 0, true, true, b"MSG2");

        let mut combined = frag1;
        combined.extend_from_slice(&frag2);

        let result = defrag.defragment(&combined);

        // Fragment layer should extract both, but PSRP parsing will likely fail
        println!("Multiple messages in one packet: {result:?}");
    }

    // =========================================================================
    // BUFFER MANAGEMENT
    // =========================================================================

    /// Test: clear_buffers() removes pending state
    #[test]
    fn test_clear_buffers() {
        let mut defrag = Defragmenter::new();

        // Start a message but don't finish it
        let incomplete = create_fragment(1, 0, true, false, b"INCOMPLETE");
        let _ = defrag.defragment(&incomplete);

        assert!(defrag.pending_count() > 0, "Should have pending buffer");

        defrag.clear_buffers();

        assert_eq!(defrag.pending_count(), 0, "Buffers should be cleared");
        println!("clear_buffers() works correctly");
    }
}
