use ironposh_winrm::soap::SoapEnvelope;
use ironposh_xml::parser::XmlDeserialize;
use std::fs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[tracing_test::traced_test]
    fn test_parse_error_response_xml_to_soap_envelope() {
        let xml_content = fs::read_to_string("tests/resources/error_response.xml")
            .expect("Failed to read error_response.xml file");

        let document =
            ironposh_xml::parser::parse(&xml_content).expect("Failed to parse XML content");
        let envelope_node = document.root_element();

        // TEST: SoapEnvelope::from_node - this is what we want to validate
        let soap_envelope = SoapEnvelope::from_node(envelope_node)
            .expect("Failed to deserialize XML into SoapEnvelope");

        // ===== VALIDATE HEADER =====
        assert!(
            soap_envelope.header.is_some(),
            "SoapEnvelope should have a header"
        );

        let header = soap_envelope.header.as_ref().unwrap();

        // Validate all header fields that should be present in error_response.xml
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

        // Validate action is for fault
        if let Some(action) = &header.value.action {
            let action_text: &str = action.value.as_ref();
            assert!(
                action_text.contains("fault"),
                "Action should contain 'fault', got: {}",
                action_text
            );
        }

        // Validate specific UUIDs from the error response
        if let Some(message_id) = &header.value.message_id {
            let uuid_str = format!("{}", message_id.value.0);
            assert!(
                uuid_str.contains("BB7AF8AE-D64A-422D-B36E-15A04FA17C5C")
                    || uuid_str.contains("bb7af8ae-d64a-422d-b36e-15a04fa17c5c"),
                "MessageID should match the error response, got: {}",
                uuid_str
            );
        }

        if let Some(relates_to) = &header.value.relates_to {
            let relates_to_text: &str = relates_to.value.as_ref();
            assert!(
                relates_to_text.contains("bead0162-a67d-424d-9e22-4a18b6aefea8"),
                "RelatesTo should contain the expected UUID, got: {}",
                relates_to_text
            );
        }

        if let Some(operation_id) = &header.value.operation_id {
            let uuid_str = format!("{}", operation_id.value.0);
            assert!(
                uuid_str.contains("fc739bfc-7556-4931-b699-677bf2c7d332")
                    || uuid_str.contains("FC739BFC-7556-4931-B699-677bf2c7d332"),
                "OperationID should match the error response, got: {}",
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

        // This error response should have a fault - but our current SoapBody doesn't support faults yet
        // For now, let's just verify that we can parse the envelope successfully without failing
        // TODO: Add fault support to SoapBody

        // Validate that normal operation fields are None (since this is an error)
        assert!(
            body.receive_response.is_none(),
            "Body should not have ReceiveResponse in error"
        );
        assert!(
            body.resource_created.is_none(),
            "Body should not have ResourceCreated in error"
        );
        assert!(body.shell.is_none(), "Body should not have Shell in error");
        assert!(
            body.command.is_none(),
            "Body should not have Command in error"
        );
        assert!(
            body.receive.is_none(),
            "Body should not have Receive in error"
        );
        assert!(
            body.command_response.is_none(),
            "Body should not have CommandResponse in error"
        );

        // Pretty print the complete parsed SOAP envelope structure
        println!("\n=== PARSED SOAP ERROR RESPONSE STRUCTURE ===");
        println!("{:#?}", soap_envelope);
        println!("=== END SOAP ERROR RESPONSE STRUCTURE ===\n");
    }
}
