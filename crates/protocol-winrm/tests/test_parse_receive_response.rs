use protocol_winrm::soap::SoapEnvelope;
use std::fs;
use xml::parser::XmlDeserialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[tracing_test::traced_test]
    fn test_parse_receive_response_xml_to_soap_envelope() {
        let xml_content = fs::read_to_string("tests/resources/receive_response.xml")
            .expect("Failed to read receive_response.xml file");

        let document = xml::parser::parse(&xml_content).expect("Failed to parse XML content");
        let envelope_node = document.root_element();

        // ONLY TEST SoapEnvelope::from_node - this is what we want to validate
        let soap_envelope = SoapEnvelope::from_node(envelope_node)
            .expect("Failed to deserialize XML into SoapEnvelope");

        // ===== VALIDATE HEADER =====
        assert!(
            soap_envelope.header.is_some(),
            "SoapEnvelope should have a header"
        );

        let header = soap_envelope.header.as_ref().unwrap();

        // Validate all header fields that should be present in receive_response.xml
        assert!(header.value.action.is_some(), "Header should have Action");
        assert!(
            header.value.message_id.is_some(),
            "Header should have MessageID"
        );
        assert!(header.value.to.is_some(), "Header should have To");
        assert!(
            header.value.relates_to.is_some(),
            "Header should have RelatesTo"
        );
        assert!(
            header.value.operation_id.is_some(),
            "Header should have OperationID"
        );
        assert!(
            header.value.sequence_id.is_some(),
            "Header should have SequenceId"
        );

        // Validate action is for ReceiveResponse
        if let Some(action) = &header.value.action {
            let action_text: &str = action.value.as_ref();
            assert!(
                action_text.contains("ReceiveResponse"),
                "Action should contain 'ReceiveResponse', got: {}",
                action_text
            );
        }

        // Validate specific UUIDs from the wire capture
        if let Some(message_id) = &header.value.message_id {
            let uuid_str = format!("{}", message_id.value.0);
            assert!(
                uuid_str.contains("6c334787-ef2c-40e4-992f-de4599ed2505")
                    || uuid_str.contains("6C334787-EF2C-40E4-992F-DE4599ED2505"),
                "MessageID should match the wire capture, got: {}",
                uuid_str
            );
        }

        if let Some(relates_to) = &header.value.relates_to {
            let relates_to_text: &str = relates_to.value.as_ref();
            assert!(
                relates_to_text.contains("87d0a667-c08e-4311-8d2d-069367f452d8"),
                "RelatesTo should contain the expected UUID, got: {}",
                relates_to_text
            );
        }

        if let Some(operation_id) = &header.value.operation_id {
            let uuid_str = format!("{}", operation_id.value.0);
            assert!(
                uuid_str.contains("672d68a1-9782-4f78-bebc-8b5db2355fda")
                    || uuid_str.contains("672D68A1-9782-4F78-BEBC-8B5DB2355FDA"),
                "OperationID should match the wire capture, got: {}",
                uuid_str
            );
        }

        if let Some(sequence_id) = &header.value.sequence_id {
            let sequence_text: &str = sequence_id.value.as_ref();
            assert_eq!(
                sequence_text.trim(),
                "1",
                "SequenceId should be '1', got: '{}'",
                sequence_text
            );
        }

        // ===== VALIDATE BODY =====
        let body = soap_envelope.body.as_ref();
        assert!(
            body.receive_response.is_some(),
            "Body should have ReceiveResponse"
        );

        let receive_response = body.receive_response.as_ref().unwrap();

        // Validate that the ReceiveResponse contains streams
        assert!(
            !receive_response.value.streams.is_empty(),
            "ReceiveResponse should contain at least one stream"
        );

        // Validate the first stream
        let first_stream = &receive_response.value.streams[0];

        // The stream should have a Name attribute set to "stdout"
        let name_attr = first_stream
            .attributes
            .iter()
            .find(|attr| matches!(attr, protocol_winrm::cores::Attribute::Name(_)));
        assert!(name_attr.is_some(), "Stream should have a Name attribute");

        if let Some(protocol_winrm::cores::Attribute::Name(name)) = name_attr {
            assert_eq!(name, "stdout", "Stream name should be 'stdout'");
        }

        // The stream should contain the base64 encoded data
        let stream_content: &str = first_stream.value.as_ref();
        assert!(!stream_content.is_empty(), "Stream should contain data");

        // Verify it contains the base64 data from the XML
        let expected_base64 = "AAAAAAAAAAMAAAAAAAAAAAMAAABnAQAAAAUQAgBsDPVb/zVH8pB752JScmEJAAAAAAAAAAAAAAAAAAAAAO+7vzxPYmogUmVmSWQ9IjAiPjxNUz48STMyIE49IlJ1bnNwYWNlU3RhdGUiPjI8L0kzMj48L01TPjwvT2JqPg==";
        assert_eq!(
            stream_content.trim(),
            expected_base64,
            "Stream content should match the expected base64 data"
        );

        // Validate that other body fields are None (since they're not in this response)
        assert!(
            body.resource_created.is_none(),
            "Body should not have ResourceCreated"
        );
        assert!(body.shell.is_none(), "Body should not have Shell");
        assert!(body.command.is_none(), "Body should not have Command");
        assert!(body.receive.is_none(), "Body should not have Receive");
        assert!(
            body.command_response.is_none(),
            "Body should not have CommandResponse"
        );

        // Pretty print the complete parsed SOAP envelope structure
        println!("\n=== PARSED SOAP ENVELOPE STRUCTURE ===");
        println!("{:#?}", soap_envelope);
        println!("=== END SOAP ENVELOPE STRUCTURE ===\n");
    }
}
