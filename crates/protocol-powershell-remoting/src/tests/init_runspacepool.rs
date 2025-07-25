use crate::{PsObject, PsProperty, PsValue};
use xml::parser::{parse, XmlDeserialize};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates an INIT_RUNSPACEPOOL object as described in the PSRP specification
    fn create_init_runspacepool() -> PsObject {
        // Create the complex nested HostInfo structure
        let host_info = create_host_info();
        
        let obj = PsObject {
            ref_id: Some(1),
            type_names: None, // Should have no associated type names
            tn_ref: None,
            props: Vec::new(),
            ms: vec![
                // MinRunspaces: I32 = 1
                PsProperty {
                    name: Some("MinRunspaces".to_string()),
                    ref_id: None,
                    value: PsValue::I32(1),
                },
                // MaxRunspaces: I32 = 1
                PsProperty {
                    name: Some("MaxRunspaces".to_string()),
                    ref_id: None,
                    value: PsValue::I32(1),
                },
                // PSThreadOptions: Object with enum structure
                PsProperty {
                    name: Some("PSThreadOptions".to_string()),
                    ref_id: None,
                    value: PsValue::Object(PsObject {
                        ref_id: Some(2),
                        type_names: Some(vec![
                            "System.Management.Automation.Runspaces.PSThreadOptions".to_string(),
                            "System.Enum".to_string(),
                            "System.ValueType".to_string(),
                            "System.Object".to_string(),
                        ]),
                        tn_ref: Some(0),
                        props: vec![
                            PsProperty {
                                name: Some("ToString".to_string()),
                                ref_id: None,
                                value: PsValue::Str("Default".to_string()),
                            },
                            PsProperty {
                                name: None,
                                ref_id: None,
                                value: PsValue::I32(0),
                            },
                        ],
                        ms: Vec::new(),
                        lst: Vec::new(),
                        dct: HashMap::new(),
                    }),
                },
                // ApartmentState: Object with enum structure
                PsProperty {
                    name: Some("ApartmentState".to_string()),
                    ref_id: None,
                    value: PsValue::Object(PsObject {
                        ref_id: Some(3),
                        type_names: Some(vec![
                            "System.Threading.ApartmentState".to_string(),
                            "System.Enum".to_string(),
                            "System.ValueType".to_string(),
                            "System.Object".to_string(),
                        ]),
                        tn_ref: Some(1),
                        props: vec![
                            PsProperty {
                                name: Some("ToString".to_string()),
                                ref_id: None,
                                value: PsValue::Str("MTA".to_string()),
                            },
                            PsProperty {
                                name: None,
                                ref_id: None,
                                value: PsValue::I32(1),
                            },
                        ],
                        ms: Vec::new(),
                        lst: Vec::new(),
                        dct: HashMap::new(),
                    }),
                },
                // HostInfo: Complex nested object
                PsProperty {
                    name: Some("HostInfo".to_string()),
                    ref_id: None,
                    value: PsValue::Object(host_info),
                },
                // ApplicationArguments: Null
                PsProperty {
                    name: Some("ApplicationArguments".to_string()),
                    ref_id: None,
                    value: PsValue::Nil,
                },
            ],
            lst: Vec::new(),
            dct: HashMap::new(),
        };

        obj
    }

    fn create_host_info() -> PsObject {
        // Create the complex hashtable for _hostDefaultData
        let mut host_data_dict = HashMap::new();
        
        // Key 9: Windows PowerShell version string
        host_data_dict.insert(
            PsValue::I32(9),
            PsValue::Object(PsObject {
                ref_id: Some(7),
                type_names: None,
                tn_ref: None,
                props: Vec::new(),
                ms: vec![
                    PsProperty {
                        name: Some("T".to_string()),
                        ref_id: None,
                        value: PsValue::Str("System.String".to_string()),
                    },
                    PsProperty {
                        name: Some("V".to_string()),
                        ref_id: None,
                        value: PsValue::Str("Windows PowerShell V2 (MS Internal Only)".to_string()),
                    },
                ],
                lst: Vec::new(),
                dct: HashMap::new(),
            }),
        );

        // Key 8: Buffer size
        host_data_dict.insert(
            PsValue::I32(8),
            PsValue::Object(create_size_object(8, 181, 98)),
        );

        // Key 7: Window size
        host_data_dict.insert(
            PsValue::I32(7),
            PsValue::Object(create_size_object(10, 120, 98)),
        );

        // Key 6: Max window size
        host_data_dict.insert(
            PsValue::I32(6),
            PsValue::Object(create_size_object(12, 120, 79)),
        );

        // Key 5: Max physical window size
        host_data_dict.insert(
            PsValue::I32(5),
            PsValue::Object(create_size_object(14, 120, 3000)),
        );

        // Key 4: Cursor size
        host_data_dict.insert(
            PsValue::I32(4),
            PsValue::Object(PsObject {
                ref_id: Some(16),
                type_names: None,
                tn_ref: None,
                props: Vec::new(),
                ms: vec![
                    PsProperty {
                        name: Some("T".to_string()),
                        ref_id: None,
                        value: PsValue::Str("System.Int32".to_string()),
                    },
                    PsProperty {
                        name: Some("V".to_string()),
                        ref_id: None,
                        value: PsValue::I32(25),
                    },
                ],
                lst: Vec::new(),
                dct: HashMap::new(),
            }),
        );

        // Key 3: Cursor position
        host_data_dict.insert(
            PsValue::I32(3),
            PsValue::Object(create_coordinates_object(17, 0, 0)),
        );

        // Key 2: Window position  
        host_data_dict.insert(
            PsValue::I32(2),
            PsValue::Object(create_coordinates_object(19, 0, 4)),
        );

        // Key 1: Foreground color
        host_data_dict.insert(
            PsValue::I32(1),
            PsValue::Object(create_console_color_object(21, 5)),
        );

        // Key 0: Background color
        host_data_dict.insert(
            PsValue::I32(0),
            PsValue::Object(create_console_color_object(22, 6)),
        );

        PsObject {
            ref_id: Some(4),
            type_names: None,
            tn_ref: None,
            props: Vec::new(),
            ms: vec![
                // _hostDefaultData
                PsProperty {
                    name: Some("_hostDefaultData".to_string()),
                    ref_id: None,
                    value: PsValue::Object(PsObject {
                        ref_id: Some(5),
                        type_names: None,
                        tn_ref: None,
                        props: Vec::new(),
                        ms: vec![
                            PsProperty {
                                name: Some("data".to_string()),
                                ref_id: None,
                                value: PsValue::Object(PsObject {
                                    ref_id: Some(6),
                                    type_names: Some(vec![
                                        "System.Collections.Hashtable".to_string(),
                                        "System.Object".to_string(),
                                    ]),
                                    tn_ref: Some(2),
                                    props: Vec::new(),
                                    ms: Vec::new(),
                                    lst: Vec::new(),
                                    dct: host_data_dict,
                                }),
                            },
                        ],
                        lst: Vec::new(),
                        dct: HashMap::new(),
                    }),
                },
                // _isHostNull
                PsProperty {
                    name: Some("_isHostNull".to_string()),
                    ref_id: None,
                    value: PsValue::Bool(false),
                },
                // _isHostUINull
                PsProperty {
                    name: Some("_isHostUINull".to_string()),
                    ref_id: None,
                    value: PsValue::Bool(false),
                },
                // _isHostRawUINull
                PsProperty {
                    name: Some("_isHostRawUINull".to_string()),
                    ref_id: None,
                    value: PsValue::Bool(false),
                },
                // _useRunspaceHost
                PsProperty {
                    name: Some("_useRunspaceHost".to_string()),
                    ref_id: None,
                    value: PsValue::Bool(false),
                },
            ],
            lst: Vec::new(),
            dct: HashMap::new(),
        }
    }

    fn create_size_object(ref_id: u32, width: i32, height: i32) -> PsObject {
        PsObject {
            ref_id: Some(ref_id),
            type_names: None,
            tn_ref: None,
            props: Vec::new(),
            ms: vec![
                PsProperty {
                    name: Some("T".to_string()),
                    ref_id: None,
                    value: PsValue::Str("System.Management.Automation.Host.Size".to_string()),
                },
                PsProperty {
                    name: Some("V".to_string()),
                    ref_id: None,
                    value: PsValue::Object(PsObject {
                        ref_id: Some(ref_id + 1),
                        type_names: None,
                        tn_ref: None,
                        props: Vec::new(),
                        ms: vec![
                            PsProperty {
                                name: Some("width".to_string()),
                                ref_id: None,
                                value: PsValue::I32(width),
                            },
                            PsProperty {
                                name: Some("height".to_string()),
                                ref_id: None,
                                value: PsValue::I32(height),
                            },
                        ],
                        lst: Vec::new(),
                        dct: HashMap::new(),
                    }),
                },
            ],
            lst: Vec::new(),
            dct: HashMap::new(),
        }
    }

    fn create_coordinates_object(ref_id: u32, x: i32, y: i32) -> PsObject {
        PsObject {
            ref_id: Some(ref_id),
            type_names: None,
            tn_ref: None,
            props: Vec::new(),
            ms: vec![
                PsProperty {
                    name: Some("T".to_string()),
                    ref_id: None,
                    value: PsValue::Str("System.Management.Automation.Host.Coordinates".to_string()),
                },
                PsProperty {
                    name: Some("V".to_string()),
                    ref_id: None,
                    value: PsValue::Object(PsObject {
                        ref_id: Some(ref_id + 1),
                        type_names: None,
                        tn_ref: None,
                        props: Vec::new(),
                        ms: vec![
                            PsProperty {
                                name: Some("x".to_string()),
                                ref_id: None,
                                value: PsValue::I32(x),
                            },
                            PsProperty {
                                name: Some("y".to_string()),
                                ref_id: None,
                                value: PsValue::I32(y),
                            },
                        ],
                        lst: Vec::new(),
                        dct: HashMap::new(),
                    }),
                },
            ],
            lst: Vec::new(),
            dct: HashMap::new(),
        }
    }

    fn create_console_color_object(ref_id: u32, color_value: i32) -> PsObject {
        PsObject {
            ref_id: Some(ref_id),
            type_names: None,
            tn_ref: None,
            props: Vec::new(),
            ms: vec![
                PsProperty {
                    name: Some("T".to_string()),
                    ref_id: None,
                    value: PsValue::Str("System.ConsoleColor".to_string()),
                },
                PsProperty {
                    name: Some("V".to_string()),
                    ref_id: None,
                    value: PsValue::I32(color_value),
                },
            ],
            lst: Vec::new(),
            dct: HashMap::new(),
        }
    }

    #[test]
    fn test_init_runspacepool_serialize() {
        let init_runspacepool = create_init_runspacepool();
        let element = init_runspacepool.to_element();
        let xml_output = element.to_string();

        println!("Serialized INIT_RUNSPACEPOOL:\n{}", xml_output);

        // Basic validation that it contains expected elements
        assert!(xml_output.contains(r#"<Obj RefId="1""#));
        assert!(xml_output.contains(r#"<MS>"#));
        assert!(xml_output.contains(r#"N="MinRunspaces""#));
        assert!(xml_output.contains(r#"N="MaxRunspaces""#));
        assert!(xml_output.contains(r#"N="PSThreadOptions""#));
        assert!(xml_output.contains(r#"N="ApartmentState""#));
        assert!(xml_output.contains(r#"N="HostInfo""#));
        assert!(xml_output.contains(r#"N="ApplicationArguments""#));
        assert!(xml_output.contains(r#"<I32"#)); // MinRunspaces and MaxRunspaces
        assert!(xml_output.contains(r#"<Nil"#)); // ApplicationArguments
        assert!(xml_output.contains(r#"<DCT>"#)); // Dictionary in HostInfo
    }

    #[test]
    fn test_init_runspacepool_deserialize() {
        // Simplified version of the example XML from the specification
        let xml_input = r#"<Obj RefId="1">
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
             <B N="_isHostNull">false</B>
             <B N="_isHostUINull">false</B>
             <B N="_isHostRawUINull">false</B>
             <B N="_useRunspaceHost">false</B>
           </MS>
         </Obj>
         <Nil N="ApplicationArguments" />
       </MS>
     </Obj>"#;

        let doc = parse(xml_input).expect("Valid XML");
        let root = doc.root_element();
        
        let deserialized_obj = PsObject::from_node(root).expect("Should deserialize successfully");

        println!("Deserialized INIT_RUNSPACEPOOL: {:#?}", deserialized_obj);

        // Verify the structure
        assert_eq!(deserialized_obj.ref_id, Some(1));
        assert_eq!(deserialized_obj.type_names, None);
        assert_eq!(deserialized_obj.tn_ref, None);
        assert!(deserialized_obj.props.is_empty());
        assert_eq!(deserialized_obj.ms.len(), 6); // MinRunspaces, MaxRunspaces, PSThreadOptions, ApartmentState, HostInfo, ApplicationArguments
        assert!(deserialized_obj.lst.is_empty());
        assert!(deserialized_obj.dct.is_empty());

        // Verify each member set property
        let ms_props: HashMap<String, &PsValue> = deserialized_obj
            .ms
            .iter()
            .filter_map(|p| p.name.as_ref().map(|n| (n.clone(), &p.value)))
            .collect();

        // Check MinRunspaces
        if let Some(PsValue::I32(v)) = ms_props.get("MinRunspaces") {
            assert_eq!(*v, 1);
        } else {
            panic!("MinRunspaces not found or wrong type");
        }

        // Check MaxRunspaces
        if let Some(PsValue::I32(v)) = ms_props.get("MaxRunspaces") {
            assert_eq!(*v, 1);
        } else {
            panic!("MaxRunspaces not found or wrong type");
        }

        // Check PSThreadOptions is an object
        if let Some(PsValue::Object(_)) = ms_props.get("PSThreadOptions") {
            println!("PSThreadOptions found as object");
        } else {
            panic!("PSThreadOptions not found or wrong type");
        }

        // Check ApartmentState is an object
        if let Some(PsValue::Object(_)) = ms_props.get("ApartmentState") {
            println!("ApartmentState found as object");
        } else {
            panic!("ApartmentState not found or wrong type");
        }

        // Check HostInfo is an object
        if let Some(PsValue::Object(_)) = ms_props.get("HostInfo") {
            println!("HostInfo found as object");
        } else {
            panic!("HostInfo not found or wrong type");
        }

        // Check ApplicationArguments is Nil
        if let Some(PsValue::Nil) = ms_props.get("ApplicationArguments") {
            println!("ApplicationArguments is Nil as expected");
        } else {
            panic!("ApplicationArguments not found or wrong type");
        }
    }

    #[test]
    fn test_init_runspacepool_roundtrip_simple() {
        // Create a simplified version for roundtrip testing
        let original = PsObject {
            ref_id: Some(1),
            type_names: None,
            tn_ref: None,
            props: Vec::new(),
            ms: vec![
                PsProperty {
                    name: Some("MinRunspaces".to_string()),
                    ref_id: None,
                    value: PsValue::I32(1),
                },
                PsProperty {
                    name: Some("MaxRunspaces".to_string()),
                    ref_id: None,
                    value: PsValue::I32(1),
                },
                PsProperty {
                    name: Some("ApplicationArguments".to_string()),
                    ref_id: None,
                    value: PsValue::Nil,
                },
            ],
            lst: Vec::new(),
            dct: HashMap::new(),
        };
        
        // Serialize
        let element = original.to_element();
        let xml_string = element.to_string();
        
        println!("Roundtrip XML:\n{}", xml_string);
        
        // Deserialize
        let doc = parse(&xml_string).expect("Valid XML");
        let root = doc.root_element();
        let deserialized = PsObject::from_node(root).expect("Should deserialize successfully");
        
        // Compare key properties
        assert_eq!(original.ref_id, deserialized.ref_id);
        assert_eq!(original.type_names, deserialized.type_names);
        assert_eq!(original.tn_ref, deserialized.tn_ref);
        assert_eq!(original.ms.len(), deserialized.ms.len());
        
        println!("Original: {:#?}", original);
        println!("Deserialized: {:#?}", deserialized);
    }
}
