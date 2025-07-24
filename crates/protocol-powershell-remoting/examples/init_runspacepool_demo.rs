use protocol_powershell_remoting::ps_objects::*;
use protocol_powershell_remoting::ps_objects::parser::PsObjectParser;
use std::io::Cursor;
use quick_xml::Writer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PowerShell INIT_RUNSPACEPOOL Message Demo ===\n");
    
    // The original XML message
    let original_xml = get_init_runspacepool_xml();
    println!("Original XML (formatted):");
    println!("{}\n", original_xml);
    
    // Parse the XML into our PsObject structure
    println!("=== Parsing XML to PsObject ===");
    let parsed_obj = PsObjectParser::parse_from_xml(&original_xml)?;
    println!("Parsed object: {:#?}\n", parsed_obj);
    
    // Demonstrate property access
    println!("=== Property Access Demo ===");
    demonstrate_property_access(&parsed_obj);
    
    // Create the same object manually (to show how to build it)
    println!("\n=== Creating Object Manually ===");
    let manual_obj = create_init_runspacepool_object();
    
    // Serialize both objects back to XML
    println!("\n=== Serializing Back to XML ===");
    
    println!("Parsed object serialized:");
    let parsed_xml = serialize_to_xml(&parsed_obj)?;
    println!("{}\n", parsed_xml);
    
    println!("Manual object serialized:");
    let manual_xml = serialize_to_xml(&manual_obj)?;
    println!("{}\n", manual_xml);
    
    // Compare the structures
    println!("=== Comparison ===");
    println!("Both objects have same RefId: {}", 
        parsed_obj.ref_id == manual_obj.ref_id);
    println!("Both objects have same number of MS properties: {}", 
        parsed_obj.ms.len() == manual_obj.ms.len());
    
    Ok(())
}

fn get_init_runspacepool_xml() -> String {
    r#"<Obj RefId="1">
  <MS>
    <I32 N="MinRunspaces">1</I32>
    <I32 N="MaxRunspaces">1</I32>
    <Obj N="PSThreadOptions" RefId="2">
      <TN RefId="0">
        <T>System.Management.Automation.Runspaces.PSThreadOptions</T>
        <T>System.Enum</T>
        <T>System.ValueType</T>
        <T>System.Object</T>
      </TN>
      <ToString>Default</ToString>
      <I32>0</I32>
    </Obj>
    <Obj N="ApartmentState" RefId="3">
      <TN RefId="1">
        <T>System.Threading.ApartmentState</T>
        <T>System.Enum</T>
        <T>System.ValueType</T>
        <T>System.Object</T>
      </TN>
      <ToString>MTA</ToString>
      <I32>1</I32>
    </Obj>
    <Obj N="HostInfo" RefId="4">
      <MS>
        <Obj N="_hostDefaultData" RefId="5">
          <MS>
            <Obj N="data" RefId="6">
              <TN RefId="2">
                <T>System.Collections.Hashtable</T>
                <T>System.Object</T>
              </TN>
              <DCT>
                <En>
                  <I32 N="Key">9</I32>
                  <Obj N="Value" RefId="7">
                    <MS>
                      <S N="T">System.String</S>
                      <S N="V">Windows PowerShell V2 (MS Internal Only)</S>
                    </MS>
                  </Obj>
                </En>
                <En>
                  <I32 N="Key">8</I32>
                  <Obj N="Value" RefId="8">
                    <MS>
                      <S N="T">System.Management.Automation.Host.Size</S>
                      <Obj N="V" RefId="9">
                        <MS>
                          <I32 N="width">181</I32>
                          <I32 N="height">98</I32>
                        </MS>
                      </Obj>
                    </MS>
                  </Obj>
                </En>
                <En>
                  <I32 N="Key">0</I32>
                  <Obj N="Value" RefId="22">
                    <MS>
                      <S N="T">System.ConsoleColor</S>
                      <I32 N="V">6</I32>
                    </MS>
                  </Obj>
                </En>
              </DCT>
            </Obj>
          </MS>
        </Obj>
        <B N="_isHostNull">false</B>
        <B N="_isHostUINull">false</B>
        <B N="_isHostRawUINull">false</B>
        <B N="_useRunspaceHost">false</B>
      </MS>
    </Obj>
    <Nil N="ApplicationArguments" />
  </MS>
</Obj>"#.to_string()
}

fn demonstrate_property_access(obj: &PsObject) {
    println!("Root object RefId: {:?}", obj.ref_id);
    println!("Number of MS properties: {}", obj.ms.len());
    
    // Find specific properties
    for (i, prop) in obj.ms.iter().enumerate() {
        println!("  Property {}: {:?} = {:?}", i, prop.name, prop.value);
        
        match &prop.value {
            PsValue::Object(nested_obj) => {
                println!("    Nested object RefId: {:?}", nested_obj.ref_id);
                if let Some(type_names) = &nested_obj.type_names {
                    println!("    Type names: {:?}", type_names.names);
                }
                if let Some(to_string) = &nested_obj.to_string {
                    println!("    ToString: {}", to_string);
                }
            }
            _ => {}
        }
    }
}

fn create_init_runspacepool_object() -> PsObject {
    let mut obj = PsObject::with_ref_id("1".to_string());
    
    // MS properties
    obj.ms.push(PsProperty::new(
        Some("MinRunspaces".to_string()),
        PsValue::Int32(1)
    ));
    
    obj.ms.push(PsProperty::new(
        Some("MaxRunspaces".to_string()),
        PsValue::Int32(1)
    ));
    
    // PSThreadOptions object
    let mut ps_thread_options = PsObject::with_ref_id("2".to_string());
    ps_thread_options.type_names = Some(PsTypeNames {
        ref_id: Some("0".to_string()),
        names: vec![
            "System.Management.Automation.Runspaces.PSThreadOptions".to_string(),
            "System.Enum".to_string(),
            "System.ValueType".to_string(),
            "System.Object".to_string(),
        ],
    });
    ps_thread_options.to_string = Some("Default".to_string());
    ps_thread_options.ms.push(PsProperty::new(None, PsValue::Int32(0)));
    
    obj.ms.push(PsProperty::new(
        Some("PSThreadOptions".to_string()),
        PsValue::Object(ps_thread_options)
    ));
    
    // ApartmentState object
    let mut apartment_state = PsObject::with_ref_id("3".to_string());
    apartment_state.type_names = Some(PsTypeNames {
        ref_id: Some("1".to_string()),
        names: vec![
            "System.Threading.ApartmentState".to_string(),
            "System.Enum".to_string(),
            "System.ValueType".to_string(),
            "System.Object".to_string(),
        ],
    });
    apartment_state.to_string = Some("MTA".to_string());
    apartment_state.ms.push(PsProperty::new(None, PsValue::Int32(1)));
    
    obj.ms.push(PsProperty::new(
        Some("ApartmentState".to_string()),
        PsValue::Object(apartment_state)
    ));
    
    // HostInfo object (simplified version)
    let mut host_info = PsObject::with_ref_id("4".to_string());
    
    // _hostDefaultData
    let mut host_default_data = PsObject::with_ref_id("5".to_string());
    
    // data hashtable
    let mut data_obj = PsObject::with_ref_id("6".to_string());
    data_obj.type_names = Some(PsTypeNames {
        ref_id: Some("2".to_string()),
        names: vec![
            "System.Collections.Hashtable".to_string(),
            "System.Object".to_string(),
        ],
    });
    
    // Add some dictionary entries
    data_obj.dct.push(PsDictionaryEntry {
        key: PsValue::Int32(9),
        value: {
            let mut value_obj = PsObject::with_ref_id("7".to_string());
            value_obj.ms.push(PsProperty::new(
                Some("T".to_string()),
                PsValue::Str("System.String".to_string())
            ));
            value_obj.ms.push(PsProperty::new(
                Some("V".to_string()),
                PsValue::Str("Windows PowerShell V2 (MS Internal Only)".to_string())
            ));
            PsValue::Object(value_obj)
        },
    });
    
    data_obj.dct.push(PsDictionaryEntry {
        key: PsValue::Int32(0),
        value: {
            let mut value_obj = PsObject::with_ref_id("22".to_string());
            value_obj.ms.push(PsProperty::new(
                Some("T".to_string()),
                PsValue::Str("System.ConsoleColor".to_string())
            ));
            value_obj.ms.push(PsProperty::new(
                Some("V".to_string()),
                PsValue::Int32(6)
            ));
            PsValue::Object(value_obj)
        },
    });
    
    host_default_data.ms.push(PsProperty::new(
        Some("data".to_string()),
        PsValue::Object(data_obj)
    ));
    
    host_info.ms.push(PsProperty::new(
        Some("_hostDefaultData".to_string()),
        PsValue::Object(host_default_data)
    ));
    
    // Host flags
    host_info.ms.push(PsProperty::new(
        Some("_isHostNull".to_string()),
        PsValue::Bool(false)
    ));
    host_info.ms.push(PsProperty::new(
        Some("_isHostUINull".to_string()),
        PsValue::Bool(false)
    ));
    host_info.ms.push(PsProperty::new(
        Some("_isHostRawUINull".to_string()),
        PsValue::Bool(false)
    ));
    host_info.ms.push(PsProperty::new(
        Some("_useRunspaceHost".to_string()),
        PsValue::Bool(false)
    ));
    
    obj.ms.push(PsProperty::new(
        Some("HostInfo".to_string()),
        PsValue::Object(host_info)
    ));
    
    // ApplicationArguments (Nil)
    obj.ms.push(PsProperty::new(
        Some("ApplicationArguments".to_string()),
        PsValue::Nil
    ));
    
    obj
}

fn serialize_to_xml(obj: &PsObject) -> Result<String, quick_xml::Error> {
    let mut output = Vec::new();
    let mut writer = Writer::new(Cursor::new(&mut output));
    obj.write_xml(&mut writer)?;
    Ok(String::from_utf8(output).unwrap_or_default())
}
