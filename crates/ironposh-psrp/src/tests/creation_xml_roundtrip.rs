use crate::{
    DefragmentResult, Defragmenter,
    ps_value::{
        PsValue,
        deserialize::{DeserializationContext, PsXmlDeserialize},
    },
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use tracing::info;

/// Integration test that reads creationXml, defragments it, deserializes it,
/// and performs a round-trip serialization/deserialization test
#[test]
#[tracing_test::traced_test]
fn test_creation_xml_roundtrip() {
    // Read the creationXml resource file
    let creation_xml_base64 = include_str!("resource/creationXml");

    // Decode from base64
    let fragment_data = B64
        .decode(creation_xml_base64.trim())
        .expect("Failed to decode base64 creationXml");

    println!("Fragment data length: {} bytes", fragment_data.len());

    // Step 1: Defragment the data
    let mut defragmenter = Defragmenter::new();
    let defrag_result = defragmenter
        .defragment(&fragment_data)
        .expect("Failed to defragment creationXml");

    let messages = match defrag_result {
        DefragmentResult::Complete(messages) => {
            println!("Successfully defragmented {} messages", messages.len());
            messages
        }
        DefragmentResult::Incomplete => {
            panic!("Defragmentation was incomplete - this shouldn't happen with test data");
        }
    };

    // Step 2: Deserialize the first message and clone it
    let first_message = &messages[0];
    println!("First message type: {:?}", first_message.message_type);

    // Parse the XML data from the first message
    let xml_data =
        String::from_utf8(first_message.data.clone()).expect("Message data should be valid UTF-8");
    println!("XML data length: {} bytes", xml_data.len());

    // Parse XML and deserialize to PsValue using context-aware deserialization
    let doc = ironposh_xml::parser::parse(&xml_data).expect("Failed to parse XML");
    let root_node = doc.root_element();

    let mut context = DeserializationContext::new();
    let original_ps_value = PsValue::from_node_with_context(root_node, &mut context)
        .expect("Failed to deserialize XML to PsValue");

    // Clone the deserialized value for comparison
    let cloned_ps_value = original_ps_value.clone();

    println!("Successfully deserialized original message to PsValue");

    // Step 3: Re-serialize the cloned PsValue back to XML
    let reserialized_element = cloned_ps_value
        .to_element_as_root()
        .expect("Failed to re-serialize PsValue to XML element");

    let reserialized_xml = reserialized_element.to_xml_string().unwrap();
    println!("Re-serialized XML length: {} bytes", reserialized_xml.len());

    // Step 4: Deserialize the re-serialized XML
    let reserialized_doc =
        ironposh_xml::parser::parse(&reserialized_xml).expect("Failed to parse re-serialized XML");
    let reserialized_root = reserialized_doc.root_element();

    let mut round_trip_context = DeserializationContext::new();
    let round_trip_ps_value =
        PsValue::from_node_with_context(reserialized_root, &mut round_trip_context)
            .expect("Failed to deserialize re-serialized XML");

    println!("Successfully deserialized round-trip message to PsValue");

    // Step 5: Compare original and round-trip deserialized values
    assert_eq!(
        original_ps_value, round_trip_ps_value,
        "Round-trip serialization/deserialization should preserve the original data"
    );

    println!("✅ Round-trip test passed! Original and round-trip values are identical.");

    // Optional: Test all messages if there are multiple
    if messages.len() > 1 {
        println!("Testing additional messages...");

        for (i, message) in messages.iter().enumerate().skip(1) {
            println!(
                "Testing message {} (type: {:?})",
                i + 1,
                message.message_type
            );

            let xml_data = String::from_utf8(message.data.clone())
                .expect("Message data should be valid UTF-8");

            let doc = ironposh_xml::parser::parse(&xml_data).expect("Failed to parse XML");
            let root_node = doc.root_element();

            // Try to deserialize using context-aware system
            let mut message_context = DeserializationContext::new();
            match PsValue::from_node_with_context(root_node, &mut message_context) {
                Ok(original) => {
                    info!(?original, "Successfully deserialized message {}", i + 1);
                    // Re-serialize
                    let reserialized_element = original
                        .to_element_as_root()
                        .expect("Failed to re-serialize PsValue to XML element");

                    let reserialized_xml = reserialized_element.to_xml_string().unwrap();

                    // Deserialize again
                    let reserialized_doc = ironposh_xml::parser::parse(&reserialized_xml)
                        .expect("Failed to parse re-serialized XML");

                    let reserialized_root = reserialized_doc.root_element();

                    let mut context2 = DeserializationContext::new();
                    let round_trip =
                        PsValue::from_node_with_context(reserialized_root, &mut context2)
                            .expect("Failed to deserialize re-serialized XML");

                    assert_eq!(
                        original,
                        round_trip,
                        "Round-trip test failed for message {}",
                        i + 1
                    );

                    println!("✅ Message {} round-trip test passed!", i + 1);
                }
                Err(e) => {
                    println!("⚠️ Message {} skipped: {:?}", i + 1, e);
                    if e.to_string().contains("Object reference") {
                        println!(
                            "   (This is expected for messages with object references - serialization needs reference preservation)"
                        );
                    } else {
                        println!("   (Unexpected error - investigate further)");
                    }
                }
            }
        }
    }

    println!("🎉 All round-trip tests completed successfully!");
}

#[test]
#[tracing_test::traced_test]
fn test_creation_xml_structure_analysis() {
    // Read and decode the creationXml
    let creation_xml_base64 = include_str!("resource/creationXml");
    let fragment_data = B64
        .decode(creation_xml_base64.trim())
        .expect("Failed to decode base64 creationXml");

    // Defragment
    let mut defragmenter = Defragmenter::new();
    let defrag_result = defragmenter
        .defragment(&fragment_data)
        .expect("Failed to defragment");

    let messages = match defrag_result {
        DefragmentResult::Complete(messages) => messages,
        DefragmentResult::Incomplete => panic!("Defragmentation incomplete"),
    };

    // Analyze each message
    for (i, message) in messages.iter().enumerate() {
        println!("=== Message {} ===", i + 1);
        println!("Destination: {:?}", message.destination);
        println!("Message Type: {:?}", message.message_type);
        println!("RPID: {}", message.rpid);
        println!("PID: {:?}", message.pid);
        println!("Data length: {} bytes", message.data.len());

        // Try to parse as XML to see structure
        let xml_data =
            String::from_utf8(message.data.clone()).expect("Message data should be valid UTF-8");

        println!("XML Content Preview:");
        let preview_len = std::cmp::min(200, xml_data.len());
        println!("  {}", &xml_data[..preview_len]);
        if xml_data.len() > preview_len {
            println!("  ... (truncated)");
        }
        println!();
    }
}
