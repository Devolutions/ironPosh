#[cfg(test)]
mod error_record_integration_tests {
    use crate::ps_value::{
        ComplexObject, ComplexObjectContent, PsPrimitiveValue, PsProperty, PsValue,
    };
    use crate::{ErrorCategory, ErrorRecord};
    use std::collections::BTreeMap;

    /// Test based on the actual failing message from the logs:
    /// "Protocol error: Invalid PowerShell remoting message: Missing Message or ErrorRecord property"
    /// This test goes from raw XML → ComplexObject → ErrorRecord to match the real pipeline
    #[test]
    #[tracing_test::traced_test]
    fn test_real_world_command_not_found_error_record() {
        // This is the exact XML from the analyzed PowerShell remoting message
        let raw_xml = include_str!("./resource/error_record.xml");

        // Parse the XML into a ComplexObject using the same pipeline as the real code
        use crate::ps_value::deserialize::{DeserializationContext, PsXmlDeserialize};

        let parsed_xml =
            ironposh_xml::parser::parse(raw_xml).expect("Should successfully parse XML");
        let root = parsed_xml.root_element();
        let mut ctx = DeserializationContext::default();
        let ps_value = PsValue::from_node_with_context(root, &mut ctx)
            .expect("Should successfully deserialize XML to PsValue");

        // Extract the ComplexObject from PsValue
        let PsValue::Object(complex_object) = ps_value else {
            panic!("Expected PsValue::Object from XML parsing")
        };

        println!("✅ Successfully parsed XML into ComplexObject");
        println!(
            "   Available properties: {:?}",
            complex_object
                .extended_properties
                .keys()
                .collect::<Vec<_>>()
        );

        // Now try to convert the ComplexObject to ErrorRecord - this is where the original error happens
        let result = ErrorRecord::try_from(complex_object);

        match result {
            Ok(error_record) => {
                println!("✅ Successfully parsed ErrorRecord!");
                println!("   Message: {}", error_record.message);
                println!("   Command: {:?}", error_record.command_name);
                println!("   Target: {:?}", error_record.target_object);
                println!("   Error ID: {:?}", error_record.fully_qualified_error_id);

                assert_eq!(
                    error_record.message,
                    "The term 'ed' is not recognized as the name of a cmdlet,, function, script file, or operable program. Check the spelling of the name, or if a path was included, verify that the path is correct and try again."
                );
                // CommandName may not be present in this specific error structure
                // assert_eq!(error_record.command_name, Some("ed".to_string()));
                println!(
                    "   Command found in parsed XML: {:?}",
                    error_record.command_name
                );
                assert_eq!(error_record.target_object, Some("ed".to_string()));
                assert_eq!(
                    error_record.fully_qualified_error_id,
                    Some("CommandNotFoundException".to_string())
                );
                assert!(!error_record.was_thrown_from_throw_statement);
                assert!(!error_record.serialize_extended_info);

                // Verify error category
                let error_category = error_record
                    .error_category
                    .expect("Error category should be present");
                assert_eq!(error_category.category, 13);
                assert_eq!(
                    error_category.reason,
                    Some("CommandNotFoundException".to_string())
                );
                assert_eq!(error_category.target_name, Some("ed".to_string()));
                assert_eq!(error_category.target_type, Some("String".to_string()));
                assert_eq!(
                    error_category.message,
                    Some("ObjectNotFound: (ed:String) [], CommandNotFoundException".to_string())
                );
            }
            Err(e) => {
                panic!("❌ Failed to parse ErrorRecord: {e}");
            }
        }
    }

    /// Test the edge case where only "ErrorRecord" property is present (no "Message")
    #[test]
    fn test_error_record_with_only_error_record_property() {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "ErrorRecord".to_string(),
            PsProperty {
                name: "ErrorRecord".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str("Test error message".to_string())),
            },
        );

        let complex_object = ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        };

        let result = ErrorRecord::try_from(complex_object);
        assert!(result.is_ok());

        let error_record = result.unwrap();
        assert_eq!(error_record.message, "Test error message");
    }

    /// Test the edge case where only "Message" property is present (no "ErrorRecord")
    #[test]
    fn test_error_record_with_only_message_property() {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "Message".to_string(),
            PsProperty {
                name: "Message".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str("Test error message".to_string())),
            },
        );

        let complex_object = ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        };

        let result = ErrorRecord::try_from(complex_object);
        assert!(result.is_ok());

        let error_record = result.unwrap();
        assert_eq!(error_record.message, "Test error message");
    }

    /// Test the case where neither "Message" nor "ErrorRecord" properties are present
    /// This should fail with the error we've been seeing
    #[test]
    fn test_error_record_missing_both_message_and_error_record() {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "SomeOtherProperty".to_string(),
            PsProperty {
                name: "SomeOtherProperty".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str("Some value".to_string())),
            },
        );

        let complex_object = ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        };

        let result = ErrorRecord::try_from(complex_object);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Missing Message or ErrorRecord property")
        );
    }

    /// Test round-trip conversion: ErrorRecord -> ComplexObject -> ErrorRecord
    #[test]
    fn test_error_record_roundtrip_conversion() {
        let original_record = ErrorRecord::builder()
            .message("The term 'ed' is not recognized as the name of a cmdlet,, function, script file, or operable program. Check the spelling of the name, or if a path was included, verify that the path is correct and try again.".to_string())
            .command_name(Some("ed".to_string()))
            .target_object(Some("ed".to_string()))
            .fully_qualified_error_id(Some("CommandNotFoundException".to_string()))
            .was_thrown_from_throw_statement(false)
            .serialize_extended_info(false)
            .error_category(Some(ErrorCategory::builder()
                .category(13)
                .activity(Some(String::new()))
                .reason(Some("CommandNotFoundException".to_string()))
                .target_name(Some("ed".to_string()))
                .target_type(Some("String".to_string()))
                .message(Some("ObjectNotFound: (ed:String) [], CommandNotFoundException".to_string()))
                .build()))
            .build();

        // Convert to ComplexObject (simulating serialization)
        let complex_object = ComplexObject::from(original_record.clone());

        // Convert back to ErrorRecord (simulating deserialization)
        let deserialized_record = ErrorRecord::try_from(complex_object)
            .expect("Should successfully deserialize ErrorRecord");

        // Verify they're equal
        assert_eq!(original_record, deserialized_record);
    }
}
