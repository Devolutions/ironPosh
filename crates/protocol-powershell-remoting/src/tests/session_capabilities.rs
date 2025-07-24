use crate::cores::{PsObject, PsProperty, PsValue};
use xml::parser::{parse, XmlDeserialize};

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a SESSION_CAPABILITY object as described in the PSRP specification
    fn create_session_capability() -> PsObject {
        let obj = PsObject {
            ref_id: Some(0),
            type_names: None, // Should have no associated type names
            tn_ref: None,
            props: Vec::new(),
            ms: vec![
                // protocolversion: Version 2.2
                PsProperty {
                    name: Some("protocolversion".to_string()),
                    ref_id: None,
                    value: PsValue::Version("2.2".to_string()),
                },
                // PSVersion: Version 2.0
                PsProperty {
                    name: Some("PSVersion".to_string()),
                    ref_id: None,
                    value: PsValue::Version("2.0".to_string()),
                },
                // SerializationVersion: Version 1.1.0.1
                PsProperty {
                    name: Some("SerializationVersion".to_string()),
                    ref_id: None,
                    value: PsValue::Version("1.1.0.1".to_string()),
                },
                // TimeZone: Binary data (base64 encoded)
                PsProperty {
                    name: Some("TimeZone".to_string()),
                    ref_id: None,
                    value: PsValue::Bytes(base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        "AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABARxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL"
                    ).expect("Valid base64")),
                },
            ],
            lst: Vec::new(),
            dct: std::collections::HashMap::new(),
        };

        obj
    }

    #[test]
    fn test_session_capability_serialize() {
        let session_cap = create_session_capability();
        let element = session_cap.to_element();
        let xml_output = element.to_string();

        println!("Serialized SESSION_CAPABILITY:\n{}", xml_output);

        // Basic validation that it contains expected elements
        assert!(xml_output.contains(r#"<Obj RefId="0""#));
        assert!(xml_output.contains(r#"<MS>"#));
        assert!(xml_output.contains(r#"N="protocolversion""#));
        assert!(xml_output.contains(r#"N="PSVersion""#));
        assert!(xml_output.contains(r#"N="SerializationVersion""#));
        assert!(xml_output.contains(r#"N="TimeZone""#));
        assert!(xml_output.contains(r#"<Version"#));
        assert!(xml_output.contains(r#"<BA"#));
    }

    #[test]
    fn test_session_capability_deserialize() {
        // Example XML from the specification
        let xml_input = r#"<Obj RefId="0">
       <MS>
         <Version N="protocolversion">2.2</Version>
         <Version N="PSVersion">2.0</Version>
         <Version N="SerializationVersion">1.1.0.1</Version>
         <BA N="TimeZone">AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABARxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL</BA>
       </MS>
     </Obj>"#;

        let doc = parse(xml_input).expect("Valid XML");
        let root = doc.root_element();
        
        let deserialized_obj = PsObject::from_node(root).expect("Should deserialize successfully");

        println!("Deserialized SESSION_CAPABILITY: {:#?}", deserialized_obj);

        // Verify the structure
        assert_eq!(deserialized_obj.ref_id, Some(0));
        assert_eq!(deserialized_obj.type_names, None);
        assert_eq!(deserialized_obj.tn_ref, None);
        assert!(deserialized_obj.props.is_empty());
        assert_eq!(deserialized_obj.ms.len(), 4);
        assert!(deserialized_obj.lst.is_empty());
        assert!(deserialized_obj.dct.is_empty());

        // Verify each member set property
        let ms_props: std::collections::HashMap<String, &PsValue> = deserialized_obj
            .ms
            .iter()
            .filter_map(|p| p.name.as_ref().map(|n| (n.clone(), &p.value)))
            .collect();

        // Check protocolversion
        if let Some(PsValue::Version(v)) = ms_props.get("protocolversion") {
            assert_eq!(v, "2.2");
        } else {
            panic!("protocolversion not found or wrong type");
        }

        // Check PSVersion
        if let Some(PsValue::Version(v)) = ms_props.get("PSVersion") {
            assert_eq!(v, "2.0");
        } else {
            panic!("PSVersion not found or wrong type");
        }

        // Check SerializationVersion
        if let Some(PsValue::Version(v)) = ms_props.get("SerializationVersion") {
            assert_eq!(v, "1.1.0.1");
        } else {
            panic!("SerializationVersion not found or wrong type");
        }

        // Check TimeZone (binary data)
        if let Some(PsValue::Bytes(bytes)) = ms_props.get("TimeZone") {
            assert!(!bytes.is_empty());
            println!("TimeZone bytes length: {}", bytes.len());
        } else {
            panic!("TimeZone not found or wrong type");
        }
    }

    #[test]
    fn test_session_capability_roundtrip() {
        // Create object, serialize, then deserialize, and compare
        let original = create_session_capability();
        
        // Serialize
        let element = original.to_element();
        let xml_string = element.to_string();
        
        println!("Roundtrip XML:\n{}", xml_string);
        
        // Deserialize
        let doc = parse(&xml_string).expect("Valid XML");
        let root = doc.root_element();
        let deserialized = PsObject::from_node(root).expect("Should deserialize successfully");
        
        // Compare key properties (note: exact equality might be tricky due to ordering)
        assert_eq!(original.ref_id, deserialized.ref_id);
        assert_eq!(original.type_names, deserialized.type_names);
        assert_eq!(original.tn_ref, deserialized.tn_ref);
        assert_eq!(original.ms.len(), deserialized.ms.len());
        
        println!("Original: {:#?}", original);
        println!("Deserialized: {:#?}", deserialized);
    }
}