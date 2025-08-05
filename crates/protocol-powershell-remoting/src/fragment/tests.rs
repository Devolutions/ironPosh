use super::*;
use crate::messages::PsObject;
use crate::{Destination, MessageType};
use uuid::Uuid;

fn create_test_message(data_size: usize) -> crate::PowerShellRemotingMessage {
    let large_data = vec![b'A'; data_size];
    let large_string = String::from_utf8(large_data).unwrap();

    let mut props = std::collections::HashMap::new();
    props.insert(
        crate::PsValue::Str("TestData".to_string()),
        crate::PsValue::Str(large_string),
    );

    let ps_object = PsObject {
        ref_id: None,
        type_names: None,
        tn_ref: None,
        props: vec![],
        ms: vec![],
        lst: vec![],
        dct: props,
    };

    crate::PowerShellRemotingMessage::new(
        Destination::Server,
        MessageType::SessionCapability,
        Uuid::new_v4(),
        Some(Uuid::new_v4()),
        &ps_object,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragment_single_message_small() {
        let mut fragmenter = Fragmenter::new(1000);
        let message = create_test_message(50);

        let fragments = fragmenter.fragment(&message);

        assert_eq!(fragments.len(), 1);
        assert!(fragments[0].start);
        assert!(fragments[0].end);
        assert_eq!(fragments[0].fragment_id, 0);
    }

    #[test]
    fn test_fragment_single_message_large() {
        let mut fragmenter = Fragmenter::new(100);
        let message = create_test_message(250);

        let fragments = fragmenter.fragment(&message);

        assert!(fragments.len() > 1);
        assert!(fragments[0].start);
        assert!(!fragments[0].end);
        assert!(fragments.last().unwrap().end);
        assert!(!fragments.last().unwrap().start);
    }

    #[test]
    fn test_fragment_multiple_messages() {
        let mut fragmenter = Fragmenter::new(200);
        let messages = vec![
            create_test_message(50),
            create_test_message(75),
            create_test_message(100),
        ];

        let request_groups = fragmenter.fragment_multiple(&messages);

        assert!(!request_groups.is_empty());
        assert!(!request_groups[0].is_empty());

        // Flatten all fragments to check overall structure
        let all_fragments: Vec<&Fragment> = request_groups
            .iter()
            .flat_map(|group| group.iter())
            .collect();

        // First fragment should start
        assert!(all_fragments[0].start);
        // Last fragment should end
        assert!(all_fragments.last().unwrap().end);

        // Each request group should have reasonable boundaries
        for group in &request_groups {
            assert!(!group.is_empty());
            assert!(group.len() <= 10); // Our heuristic limit
        }
    }

    #[test]
    fn test_defragment_single_message() {
        let mut fragmenter = Fragmenter::new(100);
        let mut defragmenter = Defragmenter::new();
        let original_message = create_test_message(250);

        // Fragment the message
        let fragments = fragmenter.fragment(&original_message);

        // Pack fragments into wire format
        let mut wire_data = Vec::new();
        for fragment in fragments {
            wire_data.extend_from_slice(&fragment.pack());
        }

        // Defragment using new API
        let result = defragmenter.defragment(&wire_data).unwrap();

        match result {
            DefragmentResult::Complete(messages) => {
                assert_eq!(messages.len(), 1);
                assert_eq!(
                    messages[0].destination as u32,
                    original_message.destination as u32
                );
                assert_eq!(
                    messages[0].message_type.value(),
                    original_message.message_type.value()
                );
            }
            DefragmentResult::Incomplete => panic!("Expected complete message"),
        }
    }

    #[test]
    fn test_defragment_partial_fragments() {
        let mut fragmenter = Fragmenter::new(100);
        let mut defragmenter = Defragmenter::new();
        let message = create_test_message(250);

        let fragments = fragmenter.fragment(&message);
        assert!(fragments.len() > 1);

        // Send only first fragment
        let first_fragment_data = fragments[0].pack();
        let result = defragmenter.defragment(&first_fragment_data).unwrap();

        // Should have no complete messages yet
        match result {
            DefragmentResult::Incomplete => {}
            DefragmentResult::Complete(_) => panic!("Should be incomplete after first fragment"),
        }
        assert_eq!(defragmenter.pending_count(), 1);

        // Send remaining fragments
        let mut remaining_data = Vec::new();
        for fragment in &fragments[1..] {
            remaining_data.extend_from_slice(&fragment.pack());
        }

        let result = defragmenter.defragment(&remaining_data).unwrap();

        // Should now have complete message
        match result {
            DefragmentResult::Complete(messages) => {
                assert_eq!(messages.len(), 1);
            }
            DefragmentResult::Incomplete => panic!("Expected complete message"),
        }
        assert_eq!(defragmenter.pending_count(), 0);
    }

    #[test]
    fn test_fragment_unpack_pack_roundtrip() {
        let original = Fragment::new(123, 456, vec![1, 2, 3, 4, 5], true, false);
        let packed = original.pack();
        let (unpacked, remaining) = Fragment::unpack(&packed).unwrap();

        assert_eq!(remaining.len(), 0);
        assert_eq!(unpacked.object_id, original.object_id);
        assert_eq!(unpacked.fragment_id, original.fragment_id);
        assert_eq!(unpacked.start, original.start);
        assert_eq!(unpacked.end, original.end);
        assert_eq!(unpacked.data, original.data);
    }

    #[test]
    fn test_defragmenter_single_complete_message() {
        let mut defragmenter = Defragmenter::new();
        let message = create_test_message(50);

        // Create a single complete fragment
        let fragment = Fragment::new(1, 0, message.clone().into_vec(), true, true);
        let packet_data = fragment.pack();

        let result = defragmenter.defragment(&packet_data).unwrap();

        match result {
            DefragmentResult::Complete(messages) => {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].destination as u32, message.destination as u32);
            }
            DefragmentResult::Incomplete => panic!("Expected complete message"),
        }

        assert_eq!(defragmenter.pending_count(), 0);
    }

    #[test]
    fn test_defragmenter_fragmented_message() {
        let mut defragmenter = Defragmenter::new();
        let mut fragmenter = Fragmenter::new(100);
        let message = create_test_message(250);

        // Fragment the message using the old fragmenter
        let fragments = fragmenter.fragment(&message);
        assert!(fragments.len() > 1);

        // Send fragments one by one
        for (i, fragment) in fragments.iter().enumerate() {
            let packet_data = fragment.pack();
            let result = defragmenter.defragment(&packet_data).unwrap();

            if i < fragments.len() - 1 {
                // Should be incomplete until the last fragment
                match result {
                    DefragmentResult::Incomplete => {}
                    DefragmentResult::Complete(_) => {
                        panic!("Unexpected complete result at fragment {}", i)
                    }
                }
                assert_eq!(defragmenter.pending_count(), 1);
            } else {
                // Last fragment should complete the message
                match result {
                    DefragmentResult::Complete(messages) => {
                        assert_eq!(messages.len(), 1);
                        assert_eq!(messages[0].destination as u32, message.destination as u32);
                    }
                    DefragmentResult::Incomplete => {
                        panic!("Expected complete message at last fragment")
                    }
                }
                assert_eq!(defragmenter.pending_count(), 0);
            }
        }
    }

    #[test]
    fn test_defragmenter_multiple_messages_mixed_packet() {
        let mut defragmenter = Defragmenter::new();

        // Create two complete single-fragment messages
        let msg1 = create_test_message(50);
        let msg2 = create_test_message(75);

        let frag1 = Fragment::new(1, 0, msg1.clone().into_vec(), true, true);
        let frag2 = Fragment::new(2, 0, msg2.clone().into_vec(), true, true);

        // Pack both fragments into a single packet (as per protocol spec)
        let mut packet_data = Vec::new();
        packet_data.extend_from_slice(&frag1.pack());
        packet_data.extend_from_slice(&frag2.pack());

        let result = defragmenter.defragment(&packet_data).unwrap();

        match result {
            DefragmentResult::Complete(messages) => {
                assert_eq!(messages.len(), 2);
            }
            DefragmentResult::Incomplete => panic!("Expected complete messages"),
        }

        assert_eq!(defragmenter.pending_count(), 0);
    }
}
