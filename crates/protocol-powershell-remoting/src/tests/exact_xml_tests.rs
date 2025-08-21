use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, PsEnums, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
    deserialize::{DeserializationContext, PsXmlDeserialize},
};
use base64::Engine;
use std::borrow::Cow;
use std::collections::BTreeMap;
use xml::parser::parse;

#[test]
fn test_session_capability_message() {
    // First message: Session capability with timezone
    let mut complex_obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    // Add extended properties (MS section)
    complex_obj.extended_properties.insert(
        "protocolversion".to_string(),
        PsProperty {
            name: "protocolversion".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Version("2.2".to_string())),
        },
    );

    complex_obj.extended_properties.insert(
        "PSVersion".to_string(),
        PsProperty {
            name: "PSVersion".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Version("2.0".to_string())),
        },
    );

    complex_obj.extended_properties.insert(
        "SerializationVersion".to_string(),
        PsProperty {
            name: "SerializationVersion".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Version("1.1.0.1".to_string())),
        },
    );

    // The base64 encoded timezone data from the example
    let timezone_data = "AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABARxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL";
    let timezone_bytes = base64::engine::general_purpose::STANDARD
        .decode(timezone_data)
        .unwrap();

    complex_obj.extended_properties.insert(
        "TimeZone".to_string(),
        PsProperty {
            name: "TimeZone".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Bytes(timezone_bytes)),
        },
    );

    // Generate XML
    let element = complex_obj.to_element_as_root().unwrap();
    let xml = element.to_xml_string().unwrap();

    println!("Generated XML:");
    println!("{}", xml);

    // Verify basic structure
    assert!(xml.contains(r#"RefId="0""#));
    assert!(xml.contains(r#"<Version N="protocolversion">2.2</Version>"#));
    assert!(xml.contains(r#"<Version N="PSVersion">2.0</Version>"#));
    assert!(xml.contains(r#"<Version N="SerializationVersion">1.1.0.1</Version>"#));
    assert!(xml.contains(r#"<BA N="TimeZone">"#));
    assert!(xml.contains("<MS>"));
}

#[test]
fn test_runspace_pool_message() {
    // Second message: Complex runspace pool configuration
    let mut complex_obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    // Add MinRunspaces property
    complex_obj.extended_properties.insert(
        "MinRunspaces".to_string(),
        PsProperty {
            name: "MinRunspaces".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(1)),
        },
    );

    // Add MaxRunspaces property
    complex_obj.extended_properties.insert(
        "MaxRunspaces".to_string(),
        PsProperty {
            name: "MaxRunspaces".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(1)),
        },
    );

    // Create PSThreadOptions enum object
    let ps_thread_options = ComplexObject {
        type_def: Some(PsType {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.Runspaces.PSThreadOptions"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        }),
        to_string: Some("Default".to_string()),
        content: ComplexObjectContent::PsEnums(PsEnums { value: 0 }),
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    complex_obj.extended_properties.insert(
        "PSThreadOptions".to_string(),
        PsProperty {
            name: "PSThreadOptions".to_string(),
            value: PsValue::Object(ps_thread_options),
        },
    );

    // Create ApartmentState enum object
    let apartment_state = ComplexObject {
        type_def: Some(PsType {
            type_names: vec![
                Cow::Borrowed("System.Threading.ApartmentState"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        }),
        to_string: Some("MTA".to_string()),
        content: ComplexObjectContent::PsEnums(PsEnums { value: 1 }),
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    complex_obj.extended_properties.insert(
        "ApartmentState".to_string(),
        PsProperty {
            name: "ApartmentState".to_string(),
            value: PsValue::Object(apartment_state),
        },
    );

    // Create the complex HostInfo object structure
    let mut host_data_dict = BTreeMap::new();

    // Add dictionary entries for host data
    for (key, value_type, value_obj) in [
        (
            9,
            "System.String",
            create_string_value_object("Windows PowerShell V2 (MS Internal Only)"),
        ),
        (
            8,
            "System.Management.Automation.Host.Size",
            create_size_value_object(181, 98),
        ),
        (
            7,
            "System.Management.Automation.Host.Size",
            create_size_value_object(120, 98),
        ),
        (
            6,
            "System.Management.Automation.Host.Size",
            create_size_value_object(120, 79),
        ),
        (
            5,
            "System.Management.Automation.Host.Size",
            create_size_value_object(120, 3000),
        ),
        (4, "System.Int32", create_int32_value_object(25)),
        (
            3,
            "System.Management.Automation.Host.Coordinates",
            create_coordinates_value_object(0, 0),
        ),
        (
            2,
            "System.Management.Automation.Host.Coordinates",
            create_coordinates_value_object(0, 4),
        ),
        (
            1,
            "System.ConsoleColor",
            create_console_color_value_object(5),
        ),
        (
            0,
            "System.ConsoleColor",
            create_console_color_value_object(6),
        ),
    ] {
        host_data_dict.insert(PsValue::Primitive(PsPrimitiveValue::I32(key)), value_obj);
    }

    let host_hashtable = ComplexObject {
        type_def: Some(PsType {
            type_names: vec![
                Cow::Borrowed("System.Collections.Hashtable"),
                Cow::Borrowed("System.Object"),
            ],
        }),
        to_string: None,
        content: ComplexObjectContent::Container(Container::Dictionary(host_data_dict)),
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    // Create _hostDefaultData object
    let mut host_default_data = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    host_default_data.extended_properties.insert(
        "data".to_string(),
        PsProperty {
            name: "data".to_string(),
            value: PsValue::Object(host_hashtable),
        },
    );

    // Create the main HostInfo object
    let mut host_info = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    host_info.extended_properties.insert(
        "_hostDefaultData".to_string(),
        PsProperty {
            name: "_hostDefaultData".to_string(),
            value: PsValue::Object(host_default_data),
        },
    );

    host_info.extended_properties.insert(
        "_isHostNull".to_string(),
        PsProperty {
            name: "_isHostNull".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Bool(false)),
        },
    );

    host_info.extended_properties.insert(
        "_isHostUINull".to_string(),
        PsProperty {
            name: "_isHostUINull".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Bool(false)),
        },
    );

    host_info.extended_properties.insert(
        "_isHostRawUINull".to_string(),
        PsProperty {
            name: "_isHostRawUINull".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Bool(false)),
        },
    );

    host_info.extended_properties.insert(
        "_useRunspaceHost".to_string(),
        PsProperty {
            name: "_useRunspaceHost".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Bool(false)),
        },
    );

    complex_obj.extended_properties.insert(
        "HostInfo".to_string(),
        PsProperty {
            name: "HostInfo".to_string(),
            value: PsValue::Object(host_info),
        },
    );

    // Add ApplicationArguments as Nil
    complex_obj.extended_properties.insert(
        "ApplicationArguments".to_string(),
        PsProperty {
            name: "ApplicationArguments".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Nil),
        },
    );

    // Generate XML
    let element = complex_obj.to_element_as_root().unwrap();
    let xml = element.to_xml_string().unwrap();

    println!("Generated XML:");
    println!("{}", xml);

    // Verify basic structure
    assert!(xml.contains(r#"RefId="1""#));
    assert!(xml.contains(r#"<I32 N="MinRunspaces">1</I32>"#));
    assert!(xml.contains(r#"<I32 N="MaxRunspaces">1</I32>"#));
    assert!(xml.contains("PSThreadOptions"));
    assert!(xml.contains("ApartmentState"));
    assert!(xml.contains("HostInfo"));
    assert!(xml.contains(r#"<Nil N="ApplicationArguments"/>"#));
}

// Helper functions to create the complex nested objects
fn create_string_value_object(value: &str) -> PsValue {
    let mut obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    obj.extended_properties.insert(
        "T".to_string(),
        PsProperty {
            name: "T".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Str("System.String".to_string())),
        },
    );

    obj.extended_properties.insert(
        "V".to_string(),
        PsProperty {
            name: "V".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Str(value.to_string())),
        },
    );

    PsValue::Object(obj)
}

fn create_size_value_object(width: i32, height: i32) -> PsValue {
    let mut size_obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    size_obj.extended_properties.insert(
        "width".to_string(),
        PsProperty {
            name: "width".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(width)),
        },
    );

    size_obj.extended_properties.insert(
        "height".to_string(),
        PsProperty {
            name: "height".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(height)),
        },
    );

    let mut wrapper_obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    wrapper_obj.extended_properties.insert(
        "T".to_string(),
        PsProperty {
            name: "T".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Str(
                "System.Management.Automation.Host.Size".to_string(),
            )),
        },
    );

    wrapper_obj.extended_properties.insert(
        "V".to_string(),
        PsProperty {
            name: "V".to_string(),
            value: PsValue::Object(size_obj),
        },
    );

    PsValue::Object(wrapper_obj)
}

fn create_coordinates_value_object(x: i32, y: i32) -> PsValue {
    let mut coords_obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    coords_obj.extended_properties.insert(
        "x".to_string(),
        PsProperty {
            name: "x".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(x)),
        },
    );

    coords_obj.extended_properties.insert(
        "y".to_string(),
        PsProperty {
            name: "y".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(y)),
        },
    );

    let mut wrapper_obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    wrapper_obj.extended_properties.insert(
        "T".to_string(),
        PsProperty {
            name: "T".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Str(
                "System.Management.Automation.Host.Coordinates".to_string(),
            )),
        },
    );

    wrapper_obj.extended_properties.insert(
        "V".to_string(),
        PsProperty {
            name: "V".to_string(),
            value: PsValue::Object(coords_obj),
        },
    );

    PsValue::Object(wrapper_obj)
}

fn create_int32_value_object(value: i32) -> PsValue {
    let mut obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    obj.extended_properties.insert(
        "T".to_string(),
        PsProperty {
            name: "T".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Str("System.Int32".to_string())),
        },
    );

    obj.extended_properties.insert(
        "V".to_string(),
        PsProperty {
            name: "V".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(value)),
        },
    );

    PsValue::Object(obj)
}

fn create_console_color_value_object(value: i32) -> PsValue {
    let mut obj = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    obj.extended_properties.insert(
        "T".to_string(),
        PsProperty {
            name: "T".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Str("System.ConsoleColor".to_string())),
        },
    );

    obj.extended_properties.insert(
        "V".to_string(),
        PsProperty {
            name: "V".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(value)),
        },
    );

    PsValue::Object(obj)
}

/// ================================================================================================
/// ROUND-TRIP TESTS: Serialize → Deserialize
/// ================================================================================================

#[test]
fn test_round_trip_session_capability() {
    // Create the original session capability object
    let mut original = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    original.extended_properties.insert(
        "protocolversion".to_string(),
        PsProperty {
            name: "protocolversion".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Version("2.2".to_string())),
        },
    );

    original.extended_properties.insert(
        "PSVersion".to_string(),
        PsProperty {
            name: "PSVersion".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Version("2.0".to_string())),
        },
    );

    let timezone_data = "AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABARxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL";
    let timezone_bytes = base64::engine::general_purpose::STANDARD
        .decode(timezone_data)
        .unwrap();

    original.extended_properties.insert(
        "TimeZone".to_string(),
        PsProperty {
            name: "TimeZone".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Bytes(timezone_bytes.clone())),
        },
    );

    // Step 1: Serialize to XML
    let element = original.to_element_as_root().unwrap();
    let xml = element.to_xml_string().unwrap();

    println!("Round-trip XML: {}", xml);

    // Step 2: Parse the XML and deserialize
    let doc = parse(&xml).expect("Failed to parse XML");
    let root = doc.root_element();
    let mut context = DeserializationContext::new();
    let deserialized =
        ComplexObject::from_node_with_context(root, &mut context).expect("Failed to deserialize");

    // Step 3: Compare key properties
    assert_eq!(
        deserialized.extended_properties.len(),
        original.extended_properties.len()
    );

    // Check protocolversion
    let proto_version = &deserialized.extended_properties["protocolversion"];
    assert_eq!(proto_version.name, "protocolversion");
    if let PsValue::Primitive(PsPrimitiveValue::Version(version)) = &proto_version.value {
        assert_eq!(version, "2.2");
    } else {
        panic!("Expected Version value for protocolversion");
    }

    // Check TimeZone bytes
    let timezone = &deserialized.extended_properties["TimeZone"];
    if let PsValue::Primitive(PsPrimitiveValue::Bytes(bytes)) = &timezone.value {
        assert_eq!(bytes, &timezone_bytes);
    } else {
        panic!("Expected Bytes value for TimeZone");
    }

    println!("✅ Session capability round-trip successful!");
}

#[test]
fn test_round_trip_enum_object() {
    // Create a PSThreadOptions enum
    let original = ComplexObject {
        type_def: Some(PsType {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.Runspaces.PSThreadOptions"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        }),
        to_string: Some("Default".to_string()),
        content: ComplexObjectContent::PsEnums(PsEnums { value: 0 }),
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    // Serialize
    let element = original.to_element_as_root().unwrap();
    let xml = element.to_xml_string().unwrap();

    println!("Enum round-trip XML: {}", xml);

    // Deserialize
    let doc = parse(&xml).expect("Failed to parse XML");
    let root = doc.root_element();
    let mut context = DeserializationContext::new();
    let deserialized =
        ComplexObject::from_node_with_context(root, &mut context).expect("Failed to deserialize");

    // Verify
    assert!(deserialized.type_def.is_some());
    let type_def = deserialized.type_def.as_ref().unwrap();
    assert_eq!(
        type_def.type_names[0].as_ref(),
        "System.Management.Automation.Runspaces.PSThreadOptions"
    );
    assert_eq!(deserialized.to_string.as_ref().unwrap(), "Default");

    if let ComplexObjectContent::PsEnums(enum_obj) = &deserialized.content {
        assert_eq!(enum_obj.value, 0);
    } else {
        panic!("Expected PsEnums content");
    }

    println!("✅ Enum object round-trip successful!");
}

#[test]
fn test_round_trip_dictionary_container() {
    // Create a dictionary container
    let mut dict = BTreeMap::new();
    dict.insert(
        PsValue::Primitive(PsPrimitiveValue::Str("key1".to_string())),
        PsValue::Primitive(PsPrimitiveValue::I32(42)),
    );
    dict.insert(
        PsValue::Primitive(PsPrimitiveValue::I32(2)),
        PsValue::Primitive(PsPrimitiveValue::Bool(true)),
    );

    let original = ComplexObject {
        type_def: Some(PsType {
            type_names: vec![
                Cow::Borrowed("System.Collections.Hashtable"),
                Cow::Borrowed("System.Object"),
            ],
        }),
        to_string: None,
        content: ComplexObjectContent::Container(Container::Dictionary(dict)),
        adapted_properties: BTreeMap::new(),
        extended_properties: BTreeMap::new(),
    };

    // Serialize
    let element = original.to_element_as_root().unwrap();
    let xml = element.to_xml_string().unwrap();

    println!("Dictionary round-trip XML: {}", xml);

    // Deserialize
    let doc = parse(&xml).expect("Failed to parse XML");
    let root = doc.root_element();
    let mut context = DeserializationContext::new();
    let deserialized =
        ComplexObject::from_node_with_context(root, &mut context).expect("Failed to deserialize");

    // Verify structure
    if let ComplexObjectContent::Container(Container::Dictionary(deserialized_dict)) =
        &deserialized.content
    {
        assert_eq!(deserialized_dict.len(), 2);

        // Check that we have the expected key-value pairs
        let has_string_key = deserialized_dict.keys().any(|k| {
            if let PsValue::Primitive(PsPrimitiveValue::Str(s)) = k {
                s == "key1"
            } else {
                false
            }
        });
        assert!(has_string_key, "Should have string key 'key1'");

        let has_int_key = deserialized_dict.keys().any(|k| {
            if let PsValue::Primitive(PsPrimitiveValue::I32(i)) = k {
                *i == 2
            } else {
                false
            }
        });
        assert!(has_int_key, "Should have int key '2'");
    } else {
        panic!("Expected Dictionary container");
    }

    println!("✅ Dictionary container round-trip successful!");
}

/// ================================================================================================
/// PREDEFINED XML DESERIALIZATION TESTS
/// ================================================================================================

#[test]
fn test_deserialize_predefined_session_capability_xml() {
    // This is the first XML message from our previous conversation
    let xml = r#"<Obj RefId="0">
   <MS>
     <Version N="protocolversion">2.2</Version>
     <Version N="PSVersion">2.0</Version>
     <Version N="SerializationVersion">1.1.0.1</Version>
     <BA N="TimeZone">AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABARxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL</BA>
   </MS>
 </Obj>"#;

    // Parse and deserialize
    let doc = parse(xml).expect("Failed to parse predefined XML");
    let root = doc.root_element();
    let mut context = DeserializationContext::new();
    let deserialized = ComplexObject::from_node_with_context(root, &mut context)
        .expect("Failed to deserialize predefined XML");

    // Verify structure
    assert_eq!(deserialized.content, ComplexObjectContent::Standard);
    assert_eq!(deserialized.extended_properties.len(), 4);

    // Check protocolversion
    let proto_version = &deserialized.extended_properties["protocolversion"];
    if let PsValue::Primitive(PsPrimitiveValue::Version(version)) = &proto_version.value {
        assert_eq!(version, "2.2");
    } else {
        panic!("Expected Version value for protocolversion");
    }

    // Check PSVersion
    let ps_version = &deserialized.extended_properties["PSVersion"];
    if let PsValue::Primitive(PsPrimitiveValue::Version(version)) = &ps_version.value {
        assert_eq!(version, "2.0");
    } else {
        panic!("Expected Version value for PSVersion");
    }

    // Check TimeZone (base64 data)
    let timezone = &deserialized.extended_properties["TimeZone"];
    if let PsValue::Primitive(PsPrimitiveValue::Bytes(bytes)) = &timezone.value {
        assert!(!bytes.is_empty());
    } else {
        panic!("Expected Bytes value for TimeZone");
    }

    println!("✅ Predefined session capability XML deserialized successfully!");
}

#[test]
fn test_deserialize_simple_enum_xml() {
    // Simple enum XML structure
    let xml = r#"<Obj RefId="2">
       <TN RefId="0">
         <T>System.Management.Automation.Runspaces.PSThreadOptions</T>
         <T>System.Enum</T>
         <T>System.ValueType</T>
         <T>System.Object</T>
       </TN>
       <ToString>Default</ToString>
       <I32>0</I32>
     </Obj>"#;

    // Parse and deserialize
    let doc = parse(xml).expect("Failed to parse enum XML");
    let root = doc.root_element();
    let mut context = DeserializationContext::new();
    let deserialized = ComplexObject::from_node_with_context(root, &mut context)
        .expect("Failed to deserialize enum XML");

    // Verify enum structure
    assert!(deserialized.type_def.is_some());
    let type_def = deserialized.type_def.as_ref().unwrap();
    assert_eq!(
        type_def.type_names[0].as_ref(),
        "System.Management.Automation.Runspaces.PSThreadOptions"
    );
    assert_eq!(deserialized.to_string.as_ref().unwrap(), "Default");

    if let ComplexObjectContent::PsEnums(enum_obj) = &deserialized.content {
        assert_eq!(enum_obj.value, 0);
    } else {
        panic!("Expected PsEnums content, got: {:?}", deserialized.content);
    }

    println!("✅ Simple enum XML deserialized successfully!");
}

#[test]
fn test_deserialize_dictionary_xml() {
    // Simple dictionary structure
    let xml = r#"<Obj RefId="4">
       <TN RefId="2">
         <T>System.Collections.Hashtable</T>
         <T>System.Object</T>
       </TN>
       <DCT>
         <En>
           <I32 N="Key">9</I32>
           <S N="Value">Windows PowerShell</S>
         </En>
         <En>
           <S N="Key">test</S>
           <I32 N="Value">42</I32>
         </En>
       </DCT>
     </Obj>"#;

    // Parse and deserialize
    let doc = parse(xml).expect("Failed to parse dictionary XML");
    let root = doc.root_element();
    let mut context = DeserializationContext::new();
    let deserialized = ComplexObject::from_node_with_context(root, &mut context)
        .expect("Failed to deserialize dictionary XML");

    // Verify dictionary structure
    if let ComplexObjectContent::Container(Container::Dictionary(dict)) = &deserialized.content {
        assert_eq!(dict.len(), 2);

        // Verify we have the expected entries
        let has_int_key = dict.keys().any(|k| {
            if let PsValue::Primitive(PsPrimitiveValue::I32(9)) = k {
                true
            } else {
                false
            }
        });
        assert!(has_int_key, "Should have integer key 9");

        let has_string_key = dict.keys().any(|k| {
            if let PsValue::Primitive(PsPrimitiveValue::Str(s)) = k {
                s == "test"
            } else {
                false
            }
        });
        assert!(has_string_key, "Should have string key 'test'");
    } else {
        panic!(
            "Expected Dictionary container, got: {:?}",
            deserialized.content
        );
    }

    println!("✅ Dictionary XML deserialized successfully!");
}

#[test]
fn test_primitive_values_round_trip() {
    let test_cases = vec![
        ("String", PsPrimitiveValue::Str("Hello World".to_string())),
        ("Boolean true", PsPrimitiveValue::Bool(true)),
        ("Boolean false", PsPrimitiveValue::Bool(false)),
        ("I32", PsPrimitiveValue::I32(-42)),
        ("U32", PsPrimitiveValue::U32(42)),
        ("I64", PsPrimitiveValue::I64(-1234567890)),
        ("Nil", PsPrimitiveValue::Nil),
        ("Version", PsPrimitiveValue::Version("1.2.3.4".to_string())),
        (
            "Guid",
            PsPrimitiveValue::Guid("12345678-1234-1234-1234-123456789012".to_string()),
        ),
        (
            "Bytes",
            PsPrimitiveValue::Bytes(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]),
        ),
    ];

    for (test_name, original_primitive) in test_cases {
        println!("Testing {}", test_name);

        // Create object with primitive content
        let original = ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::ExtendedPrimitive(original_primitive.clone()),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        };

        // Serialize
        let element = original.to_element_as_root().unwrap();
        let xml = element.to_xml_string().expect("Failed to serialize to XML");

        println!("  XML: {}", xml);

        // Deserialize
        let doc = parse(&xml).expect("Failed to parse XML");
        let root = doc.root_element();
        let mut context = DeserializationContext::new();
        let deserialized = ComplexObject::from_node_with_context(root, &mut context)
            .expect("Failed to deserialize");

        // Verify
        if let ComplexObjectContent::ExtendedPrimitive(deserialized_primitive) =
            &deserialized.content
        {
            assert_eq!(
                deserialized_primitive, &original_primitive,
                "Mismatch in {}",
                test_name
            );
        } else {
            panic!("Expected ExtendedPrimitive content for {}", test_name);
        }
    }

    println!("✅ All primitive values round-trip successful!");
}
