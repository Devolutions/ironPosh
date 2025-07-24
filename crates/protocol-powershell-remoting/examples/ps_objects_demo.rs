use protocol_powershell_remoting::ps_objects::*;
use std::io::Cursor;
use quick_xml::Writer;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Simple PowerShell Thread Options object
    println!("=== Example 1: PowerShell Thread Options ===");
    let thread_options = create_thread_options_object();
    
    let mut output = Vec::new();
    let mut writer = Writer::new(Cursor::new(&mut output));
    thread_options.write_xml(&mut writer)?;
    
    let xml = String::from_utf8(output)?;
    println!("Generated XML:\n{}", xml);
    
    // Example 2: Complex object with multiple property types
    println!("\n=== Example 2: Complex PowerShell Object ===");
    let complex_obj = create_complex_object();
    
    let mut output2 = Vec::new();
    let mut writer2 = Writer::new(Cursor::new(&mut output2));
    complex_obj.write_xml(&mut writer2)?;
    
    let xml2 = String::from_utf8(output2)?;
    println!("Generated XML:\n{}", xml2);
    
    // Example 3: Object with dictionary and array
    println!("\n=== Example 3: Object with Collections ===");
    let collections_obj = create_collections_object();
    
    let mut output3 = Vec::new();
    let mut writer3 = Writer::new(Cursor::new(&mut output3));
    collections_obj.write_xml(&mut writer3)?;
    
    let xml3 = String::from_utf8(output3)?;
    println!("Generated XML:\n{}", xml3);
    
    // Example 4: Property access demonstration
    println!("\n=== Example 4: Property Access ===");
    demonstrate_property_access();
    
    Ok(())
}

fn create_thread_options_object() -> PsObject {
    let mut obj = PsObject::with_ref_id("2".to_string());
    
    // Type names
    obj.type_names = Some(PsTypeNames {
        ref_id: Some("0".to_string()),
        names: vec![
            "System.Management.Automation.Runspaces.PSThreadOptions".to_string(),
            "System.Enum".to_string(),
            "System.ValueType".to_string(),
            "System.Object".to_string(),
        ],
    });
    
    // Member set properties
    obj.ms.push(PsProperty::new(
        Some("ToString".to_string()),
        PsValue::string("Default")
    ));
    
    obj.ms.push(PsProperty::new(
        None, // Unnamed property
        PsValue::int32(0)
    ));
    
    obj
}

fn create_complex_object() -> PsObject {
    let mut obj = PsObject::with_ref_id("5".to_string());
    
    obj.type_names = Some(PsTypeNames {
        ref_id: None,
        names: vec![
            "System.Management.Example.ComplexObject".to_string(),
            "System.Object".to_string(),
        ],
    });
    
    // Various property types in MS
    obj.ms.extend(vec![
        PsProperty::new(Some("Name".to_string()), PsValue::string("TestObject")),
        PsProperty::new(Some("Count".to_string()), PsValue::int32(42)),
        PsProperty::new(Some("IsActive".to_string()), PsValue::bool(true)),
        PsProperty::new(Some("Id".to_string()), PsValue::guid(Uuid::new_v4())),
        PsProperty::new(Some("Data".to_string()), PsValue::bytes(vec![1, 2, 3, 4, 5])),
        PsProperty::new(Some("Score".to_string()), PsValue::Double(98.5)),
        PsProperty::new(Some("Category".to_string()), PsValue::Char('A')),
        PsProperty::new(Some("Description".to_string()), PsValue::nil()),
    ]);
    
    obj
}

fn create_collections_object() -> PsObject {
    let mut obj = PsObject::with_ref_id("10".to_string());
    
    obj.type_names = Some(PsTypeNames {
        ref_id: None,
        names: vec![
            "System.Collections.Example".to_string(),
            "System.Object".to_string(),
        ],
    });
    
    // Array in LST
    obj.lst.extend(vec![
        PsProperty::new(None, PsValue::string("First Item")),
        PsProperty::new(None, PsValue::string("Second Item")),
        PsProperty::new(None, PsValue::int32(123)),
        PsProperty::new(None, PsValue::bool(false)),
    ]);
    
    // Dictionary entries
    obj.dct.extend(vec![
        PsDictionaryEntry {
            key: PsValue::string("key1"),
            value: PsValue::string("value1"),
        },
        PsDictionaryEntry {
            key: PsValue::string("key2"),
            value: PsValue::int32(456),
        },
        PsDictionaryEntry {
            key: PsValue::int32(789),
            value: PsValue::bool(true),
        },
    ]);
    
    obj
}

fn demonstrate_property_access() {
    let mut obj = PsObject::new();
    
    // Add some properties
    obj.ms.extend(vec![
        PsProperty::new(Some("CaseSensitive".to_string()), PsValue::string("Exact Match")),
        PsProperty::new(Some("caseinsensitive".to_string()), PsValue::int32(789)),
        PsProperty::new(Some("MixedCase".to_string()), PsValue::bool(true)),
    ]);
    
    println!("Property access demonstration:");
    
    // Case sensitive access
    if let Some(value) = obj.get_property("CaseSensitive", true) {
        println!("  Case sensitive 'CaseSensitive': {:?}", value);
    }
    
    // This should fail (case sensitive)
    if obj.get_property("casesensitive", true).is_none() {
        println!("  Case sensitive 'casesensitive': Not found (expected)");
    }
    
    // Case insensitive access
    if let Some(value) = obj.get_property("CASESENSITIVE", false) {
        println!("  Case insensitive 'CASESENSITIVE': {:?}", value);
    }
    
    if let Some(value) = obj.get_property("MIXEDCASE", false) {
        println!("  Case insensitive 'MIXEDCASE': {:?}", value);
    }
    
    // Access via PSProperty
    if let Some(prop) = obj.get_ps_property("caseinsensitive", false) {
        println!("  PSProperty 'caseinsensitive' name: {:?}, value: {:?}", prop.name, prop.value);
    }
}
