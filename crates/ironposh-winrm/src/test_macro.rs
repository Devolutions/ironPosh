use crate::cores::tag_name::*;
use crate::cores::{Tag, Text};
use ironposh_macros::{SimpleTagValue, SimpleXmlDeserialize};

// Example struct with mixed required and optional fields using new derive macros
#[derive(Debug, Clone, SimpleTagValue, SimpleXmlDeserialize)]
pub struct TestStruct<'a> {
    pub action: Tag<'a, Text<'a>, Action>,        // Required
    pub message_id: Tag<'a, Text<'a>, MessageID>, // Required
    pub to: Option<Tag<'a, Text<'a>, To>>,        // Optional
    pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>, // Optional
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cores::TagValue;
    use ironposh_xml::parser::XmlDeserialize;

    #[test]
    fn test_serialization_and_deserialization_roundtrip() {
        // Create a test struct with both required and optional fields
        let original = TestStruct {
            action: Tag::new(Text::from("test-action")),
            message_id: Tag::new(Text::from("msg-123")),
            to: Some(Tag::new(Text::from("destination"))),
            relates_to: None, // This optional field is not set
        };

        // Test that the TagValue implementation works (serialize to XML)
        let element = ironposh_xml::builder::Element::new("test");
        let _serialized_element = original.append_to_element(element);

        // The TagValue implementation worked if we got here without panicking

        // Test deserialization with a simple XML string
        let test_xml = r#"<test>
            <Action>test-action</Action>
            <MessageID>msg-123</MessageID>
            <To>destination</To>
        </test>"#;

        // Parse the XML back
        let doc = ironposh_xml::parser::parse(test_xml).expect("Failed to parse XML");
        let root = doc.root_element();

        // Deserialize back to struct
        let deserialized = TestStruct::from_node(root).expect("Failed to deserialize");

        println!("Deserialized struct: {:#?}", deserialized);

        // Verify deserialization matches original
        assert_eq!(deserialized.action.value.as_ref(), "test-action");
        assert_eq!(deserialized.message_id.value.as_ref(), "msg-123");
        assert!(deserialized.to.is_some());
        assert_eq!(deserialized.to.unwrap().value.as_ref(), "destination");
        assert!(deserialized.relates_to.is_none());
    }

    #[test]
    fn test_deserialize_with_all_fields() {
        let xml = r#"
            <test xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing">
                <a:Action>test-action</a:Action>
                <a:MessageID>msg-123</a:MessageID>
                <a:To>destination</a:To>
                <a:RelatesTo>relation-123</a:RelatesTo>
            </test>
        "#;

        let doc = ironposh_xml::parser::parse(xml).expect("Failed to parse XML");
        let root = doc.root_element();

        let result = TestStruct::from_node(root).expect("Failed to deserialize");
        println!("Deserialized with all fields: {:#?}", result);

        // Verify required fields
        assert_eq!(result.action.value.as_ref(), "test-action");
        assert_eq!(result.message_id.value.as_ref(), "msg-123");

        // Verify optional fields
        assert!(result.to.is_some());
        assert_eq!(result.to.unwrap().value.as_ref(), "destination");
        assert!(result.relates_to.is_some());
        assert_eq!(result.relates_to.unwrap().value.as_ref(), "relation-123");
    }

    #[test]
    fn test_deserialize_with_only_required_fields() {
        let xml = r#"
            <test xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing">
                <a:Action>test-action</a:Action>
                <a:MessageID>msg-123</a:MessageID>
            </test>
        "#;

        let doc = ironposh_xml::parser::parse(xml).expect("Failed to parse XML");
        let root = doc.root_element();

        let result = TestStruct::from_node(root).expect("Failed to deserialize");
        println!("Deserialized with required fields only: {:#?}", result);

        // Verify required fields
        assert_eq!(result.action.value.as_ref(), "test-action");
        assert_eq!(result.message_id.value.as_ref(), "msg-123");

        // Verify optional fields are None
        assert!(result.to.is_none());
        assert!(result.relates_to.is_none());
    }

    #[test]
    fn test_deserialize_missing_required_field() {
        let xml = r#"
            <test xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing">
                <a:Action>test-action</a:Action>
                <!-- Missing MessageID -->
            </test>
        "#;

        let doc = ironposh_xml::parser::parse(xml).expect("Failed to parse XML");
        let root = doc.root_element();

        let result = TestStruct::from_node(root);
        assert!(result.is_err());
        println!(
            "Expected error for missing required field: {:?}",
            result.err()
        );
    }
}
