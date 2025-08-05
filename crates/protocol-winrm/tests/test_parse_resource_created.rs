use protocol_winrm::soap::SoapEnvelope;
use std::fs;
use xml::parser::XmlDeserialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resource_created_xml_to_soap_envelope() {
        let xml_content = fs::read_to_string("tests/resources/resource_created.xml")
            .expect("Failed to read resource_created.xml file");

        let document = xml::parser::parse(&xml_content).expect("Failed to parse XML content");
        let envelope_node = document.root_element();

        // ONLY TEST SoapEnvelope::from_node - this is what we want to validate
        let soap_envelope = SoapEnvelope::from_node(envelope_node)
            .expect("Failed to deserialize XML into SoapEnvelope");

        // Validate the deserialization worked correctly
        assert!(
            soap_envelope.header.is_some(),
            "SoapEnvelope should have a header"
        );

        // Validate header fields were parsed correctly
        let header = soap_envelope.header.as_ref().unwrap();
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

        // Validate body was parsed
        // The body should contain the parsed content
        // Note: We can't easily inspect the body content without knowing its exact structure,
        // but we can verify the body exists and was parsed

        let body = soap_envelope.body.as_ref();
        assert!(
            body.resource_created.is_some(),
            "Body should have ResourceCreated"
        );
        let resource_created = body.resource_created.as_ref().unwrap().as_ref();

        // ResourceCreated now has required fields (Tag<..> not Option<Tag<..>>)
        // So we can access them directly without checking is_some()
        let _address = &resource_created.address; // This is a Tag<'a, Text<'a>, Address>
        let _reference_parameters = &resource_created.reference_parameters; // This is a Tag<'a, ReferenceParametersValue<'a>, ReferenceParameters>

        // Validate ReferenceParameters content
        // ReferenceParametersValue also has required fields now
        let ref_params = &resource_created.reference_parameters.value;
        let _resource_uri = &ref_params.resource_uri; // This is a Tag<'a, Text<'a>, ResourceURI>
        let _selector_set = &ref_params.selector_set; // This is a Tag<'a, SelectorSetValue, SelectorSet>

        // Validate that Shell element is also present (it should be ignored by ResourceCreated parser)
        assert!(body.shell.is_some(), "Body should also have Shell element");
    }
}
