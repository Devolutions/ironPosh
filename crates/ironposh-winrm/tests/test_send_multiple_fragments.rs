use ironposh_winrm::{
    cores::{Attribute, Send, Tag, Text, tag_name::Stream},
    rsp::send::SendValue,
};
use ironposh_xml::builder::Element;
use uuid::Uuid;

/// Test that SendValue correctly generates multiple <rsp:Stream> elements
/// when sending fragmented PSRP host responses
#[test]
fn test_send_with_multiple_stream_fragments() {
    // Simulate 3 base64-encoded PSRP fragments
    let fragments = [
        "AAAAAAAAAA==".to_string(), // Fragment 1
        "BBBBBBBBBB==".to_string(), // Fragment 2
        "CCCCCCCCCC==".to_string(), // Fragment 3
    ];

    let command_id = Uuid::new_v4();

    // Create Stream tags for each fragment
    let streams: Vec<Tag<Text, Stream>> = fragments
        .iter()
        .map(|fragment| {
            Tag::from_name(Stream)
                .with_value(Text::from(fragment.as_str()))
                .with_attribute(Attribute::Name("stdin".into()))
        })
        .collect();

    // Create SendValue with multiple streams
    let send_value = SendValue::builder().streams(streams).build();

    // Create Send tag with CommandId
    let send_tag = Tag::from_name(Send)
        .with_value(send_value)
        .with_attribute(Attribute::CommandId(command_id))
        .with_declaration(ironposh_winrm::cores::Namespace::WsmanShell);

    let element: Element = send_tag.into_element();
    let xml_string = element.to_xml_string().unwrap();

    println!("Generated Send XML: {xml_string}");

    // Verify the XML structure
    assert!(xml_string.contains("<rsp:Send"), "Should have Send element");
    assert!(
        xml_string.contains(&format!(
            "CommandId=\"{}\"",
            command_id.to_string().to_uppercase()
        )),
        "Should have CommandId attribute"
    );

    // Verify we have exactly 3 Stream elements
    let stream_count = xml_string.matches("<rsp:Stream").count();
    assert_eq!(
        stream_count, 3,
        "Should have exactly 3 Stream elements (one per fragment)"
    );

    // Verify each fragment is in its own Stream element
    assert!(
        xml_string.contains("AAAAAAAAAA=="),
        "Should contain first fragment"
    );
    assert!(
        xml_string.contains("BBBBBBBBBB=="),
        "Should contain second fragment"
    );
    assert!(
        xml_string.contains("CCCCCCCCCC=="),
        "Should contain third fragment"
    );

    // Verify Stream elements have Name="stdin" attribute
    let stdin_count = xml_string.matches("Name=\"stdin\"").count();
    assert_eq!(
        stdin_count, 3,
        "Each Stream should have Name=\"stdin\" attribute"
    );
}

/// Test Send without CommandId (shell-level send)
#[test]
fn test_send_without_command_id() {
    let fragments = ["FRAGMENT1==".to_string(), "FRAGMENT2==".to_string()];

    let streams: Vec<Tag<Text, Stream>> = fragments
        .iter()
        .map(|fragment| {
            Tag::from_name(Stream)
                .with_value(Text::from(fragment.as_str()))
                .with_attribute(Attribute::Name("stdin".into()))
        })
        .collect();

    let send_value = SendValue::builder().streams(streams).build();

    let send_tag = Tag::from_name(Send)
        .with_value(send_value)
        .with_declaration(ironposh_winrm::cores::Namespace::WsmanShell);

    let element: Element = send_tag.into_element();
    let xml_string = element.to_xml_string().unwrap();

    println!("Generated Shell-level Send XML: {xml_string}");

    assert!(xml_string.contains("<rsp:Send"), "Should have Send element");
    assert!(
        !xml_string.contains("CommandId="),
        "Should NOT have CommandId attribute"
    );

    let stream_count = xml_string.matches("<rsp:Stream").count();
    assert_eq!(stream_count, 2, "Should have exactly 2 Stream elements");
}

/// Test that large fragments (simulating >512KB response) are properly separated
#[test]
fn test_send_large_response_multiple_fragments() {
    // Simulate a large host response that was fragmented into 10 fragments
    // Each fragment would be close to the max fragment size (~512KB - overhead)
    let large_fragment_data = "A".repeat(1000); // Simplified for test
    let fragments: Vec<String> = (0..10)
        .map(|i| format!("{large_fragment_data}{i:04}=="))
        .collect();

    let command_id = Uuid::new_v4();

    let streams: Vec<Tag<Text, Stream>> = fragments
        .iter()
        .map(|fragment| {
            Tag::from_name(Stream)
                .with_value(Text::from(fragment.as_str()))
                .with_attribute(Attribute::Name("stdin".into()))
        })
        .collect();

    let send_value = SendValue::builder().streams(streams).build();

    let send_tag = Tag::from_name(Send)
        .with_value(send_value)
        .with_attribute(Attribute::CommandId(command_id))
        .with_declaration(ironposh_winrm::cores::Namespace::WsmanShell);

    let element: Element = send_tag.into_element();
    let xml_string = element.to_xml_string().unwrap();

    println!(
        "Generated large response Send XML (length: {} bytes)",
        xml_string.len()
    );

    // Verify we have 10 separate Stream elements
    let stream_count = xml_string.matches("<rsp:Stream").count();
    assert_eq!(
        stream_count, 10,
        "Should have 10 Stream elements for large fragmented response"
    );

    // Verify each fragment is present
    for i in 0..10 {
        assert!(
            xml_string.contains(&format!("{i:04}==")),
            "Should contain fragment {i}"
        );
    }

    // The total envelope size should be reasonable (not concatenated into single huge stream)
    // This is a rough check - in practice, we'd verify it stays under max_envelope_size
    println!("Total XML size: {} bytes", xml_string.len());
}
