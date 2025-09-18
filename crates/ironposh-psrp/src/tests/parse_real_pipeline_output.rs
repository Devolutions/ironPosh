use crate::{
    ps_value::deserialize::PsXmlDeserialize,
    ps_value::{
        ComplexObjectContent, Container, PsPrimitiveValue, PsValue,
        deserialize::DeserializationContext,
    },
};

const PIPELINE_OUTPUT: &str = r#"
<Obj RefId="0"><TN RefId="0"><T>System.IO.DirectoryInfo</T><T>System.IO.FileSystemInfo</T><T>System.MarshalByRefObject</T><T>System.Object</T></TN><ToString>ADMF</ToString><Props><S N="Name">ADMF</S><S N="FullName">C:\Users\Administrator\Documents\ADMF</S><S N="Parent">Documents</S><B N="Exists">true</B><S N="Root">C:\</S><S N="Extension"></S><DT N="CreationTime">2023-08-31T14:40:37.0592148-04:00</DT><DT N="CreationTimeUtc">2023-08-31T18:40:37.0592148Z</DT><DT N="LastAccessTime">2023-08-31T14:40:41.2433837-04:00</DT><DT N="LastAccessTimeUtc">2023-08-31T18:40:41.2433837Z</DT><DT N="LastWriteTime">2023-08-31T14:40:40.6107368-04:00</DT><DT N="LastWriteTimeUtc">2023-08-31T18:40:40.6107368Z</DT><S N="Attributes">Directory</S></Props><MS><S N="PSPath">Microsoft.PowerShell.Core\FileSystem::C:\Users\Administrator\Documents\ADMF</S><S N="PSParentPath">Microsoft.PowerShell.Core\FileSystem::C:\Users\Administrator\Documents</S><S N="PSChildName">ADMF</S><Obj N="PSDrive" RefId="1"><TN RefId="1"><T>System.Management.Automation.PSDriveInfo</T><T>System.Object</T></TN><ToString>C</ToString><Props><S N="CurrentLocation">Users\Administrator\Documents</S><S N="Name">C</S><S N="Provider">Microsoft.PowerShell.Core\FileSystem</S><S N="Root">C:\</S><S N="Description">System</S><Nil N="MaximumSize" /><Obj N="Credential" RefId="2"><TN RefId="2"><T>System.Management.Automation.PSCredential</T><T>System.Object</T></TN><ToString>System.Management.Automation.PSCredential</ToString><Props><Nil N="UserName" /><Nil N="Password" /></Props></Obj><Nil N="DisplayRoot" /></Props><MS><U64 N="Used">66251223040</U64><U64 N="Free">70923485184</U64></MS></Obj><Obj N="PSProvider" RefId="3"><TN RefId="3"><T>System.Management.Automation.ProviderInfo</T><T>System.Object</T></TN><ToString>Microsoft.PowerShell.Core\FileSystem</ToString><Props><S N="ImplementingType">Microsoft.PowerShell.Commands.FileSystemProvider</S><S N="HelpFile">System.Management.Automation.dll-Help.xml</S><S N="Name">FileSystem</S><S N="PSSnapIn">Microsoft.PowerShell.Core</S><S N="ModuleName">Microsoft.PowerShell.Core</S><Nil N="Module" /><S N="Description"></S><S N="Capabilities">Filter, ShouldProcess, Credentials</S><S N="Home">C:\Users\Administrator</S><Obj N="Drives" RefId="4"><TN RefId="4"><T>System.Collections.ObjectModel.Collection`1[[System.Management.Automation.PSDriveInfo, System.Management.Automation, Version=3.0.0.0, Culture=neutral, PublicKeyToken=31bf3856ad364e35]]</T><T>System.Object</T></TN><LST><Ref RefId="1" /><S>A</S><S>D</S></LST></Obj></Props></Obj><B N="PSIsContainer">true</B><S N="Mode">d-----</S><S N="BaseName">ADMF</S><Obj N="Target" RefId="5"><TN RefId="5"><T>System.Collections.Generic.List`1[[System.String, mscorlib, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089]]</T><T>System.Object</T></TN><LST /></Obj><Nil N="LinkType" /></MS></Obj>
"#;

#[test]
fn test_parse_real_pipeline_output() {
    // Parse the XML
    let parsed = ironposh_xml::parser::parse(PIPELINE_OUTPUT).expect("Failed to parse XML");
    let root = parsed.root_element();

    // Create deserialization context
    let mut context = DeserializationContext::default();

    // Parse to PsValue
    let ps_value = PsValue::from_node_with_context(root, &mut context)
        .expect("Failed to parse XML to PsValue");

    // Verify that we got a complex object
    let complex_obj = ps_value.as_object().expect("Expected complex object");

    // Test basic structure
    assert!(
        complex_obj.type_def.is_some(),
        "Should have type definition"
    );

    let type_def = complex_obj.type_def.as_ref().unwrap();
    assert!(!type_def.type_names.is_empty(), "Should have type names");
    assert_eq!(
        type_def.type_names[0], "System.IO.DirectoryInfo",
        "First type should be DirectoryInfo"
    );

    // Test ToString
    assert_eq!(
        complex_obj.to_string,
        Some("ADMF".to_string()),
        "ToString should be ADMF"
    );

    // Test adapted properties
    assert!(
        !complex_obj.adapted_properties.is_empty(),
        "Should have adapted properties"
    );

    // Check specific properties
    let name_prop = complex_obj
        .adapted_properties
        .get("Name")
        .expect("Should have Name property");
    if let PsValue::Primitive(PsPrimitiveValue::Str(name)) = &name_prop.value {
        assert_eq!(name, "ADMF", "Name should be ADMF");
    } else {
        panic!("Name property should be a string");
    }

    let full_name_prop = complex_obj
        .adapted_properties
        .get("FullName")
        .expect("Should have FullName property");
    if let PsValue::Primitive(PsPrimitiveValue::Str(full_name)) = &full_name_prop.value {
        assert_eq!(
            full_name, "C:\\Users\\Administrator\\Documents\\ADMF",
            "FullName should match"
        );
    } else {
        panic!("FullName property should be a string");
    }

    let exists_prop = complex_obj
        .adapted_properties
        .get("Exists")
        .expect("Should have Exists property");
    if let PsValue::Primitive(PsPrimitiveValue::Bool(exists)) = &exists_prop.value {
        assert!(exists, "Exists should be true");
    } else {
        panic!("Exists property should be a boolean");
    }

    // Test extended properties (MS section)
    assert!(
        !complex_obj.extended_properties.is_empty(),
        "Should have extended properties"
    );

    let ps_path_prop = complex_obj
        .extended_properties
        .get("PSPath")
        .expect("Should have PSPath property");
    if let PsValue::Primitive(PsPrimitiveValue::Str(ps_path)) = &ps_path_prop.value {
        assert_eq!(
            ps_path,
            "Microsoft.PowerShell.Core\\FileSystem::C:\\Users\\Administrator\\Documents\\ADMF",
            "PSPath should match"
        );
    } else {
        panic!("PSPath property should be a string");
    }

    // Test nested objects (PSDrive)
    let ps_drive_prop = complex_obj
        .extended_properties
        .get("PSDrive")
        .expect("Should have PSDrive property");
    if let PsValue::Object(ps_drive_obj) = &ps_drive_prop.value {
        assert!(
            ps_drive_obj.type_def.is_some(),
            "PSDrive should have type definition"
        );
        let ps_drive_type = ps_drive_obj.type_def.as_ref().unwrap();
        assert_eq!(
            ps_drive_type.type_names[0], "System.Management.Automation.PSDriveInfo",
            "PSDrive type should be correct"
        );

        // Test PSDrive ToString
        assert_eq!(
            ps_drive_obj.to_string,
            Some("C".to_string()),
            "PSDrive ToString should be C"
        );

        // Test PSDrive properties
        let name_prop = ps_drive_obj
            .adapted_properties
            .get("Name")
            .expect("PSDrive should have Name property");
        if let PsValue::Primitive(PsPrimitiveValue::Str(name)) = &name_prop.value {
            assert_eq!(name, "C", "PSDrive Name should be C");
        } else {
            panic!("PSDrive Name property should be a string");
        }
    } else {
        panic!("PSDrive property should be an object");
    }

    // Test object references (in the Drives collection)
    let ps_provider_prop = complex_obj
        .extended_properties
        .get("PSProvider")
        .expect("Should have PSProvider property");
    if let PsValue::Object(ps_provider_obj) = &ps_provider_prop.value {
        let drives_prop = ps_provider_obj
            .adapted_properties
            .get("Drives")
            .expect("PSProvider should have Drives property");
        if let PsValue::Object(drives_obj) = &drives_prop.value {
            if let ComplexObjectContent::Container(Container::List(drives_list)) =
                &drives_obj.content
            {
                assert!(!drives_list.is_empty(), "Drives list should not be empty");

                // The first item should be a reference to the PSDrive object
                if let PsValue::Object(ref_obj) = &drives_list[0] {
                    // This should be the same PSDrive object referenced earlier
                    assert_eq!(
                        ref_obj.to_string,
                        Some("C".to_string()),
                        "Referenced PSDrive should have ToString C"
                    );
                } else {
                    panic!("First drive should be an object");
                }
            } else {
                panic!("Drives should be a list container");
            }
        } else {
            panic!("Drives property should be an object");
        }
    } else {
        panic!("PSProvider property should be an object");
    }

    println!("Successfully parsed complex PowerShell DirectoryInfo object!");
    println!(
        "Object type: {:?}",
        complex_obj.type_def.as_ref().unwrap().type_names[0]
    );
    println!("ToString: {:?}", complex_obj.to_string);
    println!(
        "Adapted properties count: {}",
        complex_obj.adapted_properties.len()
    );
    println!(
        "Extended properties count: {}",
        complex_obj.extended_properties.len()
    );
}

#[test]
fn test_parse_real_pipeline_output_detailed_inspection() {
    // Parse the XML
    let parsed = ironposh_xml::parser::parse(PIPELINE_OUTPUT).expect("Failed to parse XML");
    let root = parsed.root_element();

    // Create deserialization context
    let mut context = DeserializationContext::default();

    // Parse to PsValue
    let ps_value = PsValue::from_node_with_context(root, &mut context)
        .expect("Failed to parse XML to PsValue");

    // Print detailed structure for inspection
    match &ps_value {
        PsValue::Object(obj) => {
            println!("=== COMPLEX OBJECT STRUCTURE ===");

            if let Some(type_def) = &obj.type_def {
                println!("Type names: {:?}", type_def.type_names);
            }

            println!("ToString: {:?}", obj.to_string);

            println!("\nAdapted Properties ({}):", obj.adapted_properties.len());
            for (name, prop) in &obj.adapted_properties {
                println!("  {}: {:?}", name, classify_ps_value(&prop.value));
            }

            println!("\nExtended Properties ({}):", obj.extended_properties.len());
            for (name, prop) in &obj.extended_properties {
                println!("  {}: {:?}", name, classify_ps_value(&prop.value));
            }

            // Check if we have any container content
            match &obj.content {
                ComplexObjectContent::Standard => println!("\nContent: Standard"),
                ComplexObjectContent::ExtendedPrimitive(prim) => {
                    println!("\nContent: ExtendedPrimitive({prim:?})")
                }
                ComplexObjectContent::Container(container) => {
                    println!("\nContent: Container({container:?})");
                }
                ComplexObjectContent::PsEnums(enums) => {
                    println!("\nContent: Enum({})", enums.value)
                }
            }
        }
        PsValue::Primitive(prim) => {
            println!("=== PRIMITIVE VALUE ===");
            println!("{prim:?}");
        }
    }
}

fn classify_ps_value(value: &PsValue) -> String {
    match value {
        PsValue::Primitive(prim) => format!("Primitive({prim:?})"),
        PsValue::Object(obj) => {
            let type_name = obj
                .type_def
                .as_ref()
                .and_then(|t| t.type_names.first())
                .map(|s| s.as_ref())
                .unwrap_or("Unknown");

            let content_type = match &obj.content {
                ComplexObjectContent::Standard => "Standard",
                ComplexObjectContent::ExtendedPrimitive(_) => "ExtendedPrimitive",
                ComplexObjectContent::Container(container) => match container {
                    Container::List(_) => "List",
                    Container::Dictionary(_) => "Dictionary",
                    Container::Stack(_) => "Stack",
                    Container::Queue(_) => "Queue",
                },
                ComplexObjectContent::PsEnums(_) => "Enum",
            };

            format!(
                "Object({}, content: {}, props: {}, ext_props: {})",
                type_name,
                content_type,
                obj.adapted_properties.len(),
                obj.extended_properties.len()
            )
        }
    }
}
