use crate::{PsObject, PsProperty, PsValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PSThreadOptions {
    Default = 0,
    UseNewThread = 1,
    ReuseThread = 2,
    UseCurrentThread = 3,
}

impl From<PSThreadOptions> for PsObject {
    fn from(option: PSThreadOptions) -> Self {
        PsObject {
            type_names: Some(vec![
                "System.Management.Automation.Runspaces.PSThreadOptions".to_string(),
                "System.Enum".to_string(),
                "System.ValueType".to_string(),
                "System.Object".to_string(),
            ]),
            to_string: Some(match option {
                PSThreadOptions::Default => "Default".to_string(),
                PSThreadOptions::UseNewThread => "UseNewThread".to_string(),
                PSThreadOptions::ReuseThread => "ReuseThread".to_string(),
                PSThreadOptions::UseCurrentThread => "UseCurrentThread".to_string(),
            }),
            enum_value: Some(option as i32),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ps_thread_options_serialization() {
        let option = PSThreadOptions::Default;
        let ps_obj = PsObject::from(option);
        let xml = ps_obj.to_element().to_string();
        
        println!("Generated XML:");
        println!("{}", xml);
        
        // Expected XML from working example:
        // <Obj N="PSThreadOptions" RefId="1">
        //   <TN RefId="0">
        //     <T>System.Management.Automation.Runspaces.PSThreadOptions</T>
        //     <T>System.Enum</T>
        //     <T>System.ValueType</T>
        //     <T>System.Object</T>
        //   </TN>
        //   <ToString>Default</ToString>
        //   <I32>0</I32>
        // </Obj>
        
        // Check basic structure
        assert!(xml.contains("<ToString>Default</ToString>"));
        assert!(xml.contains("<I32>0</I32>"));
        assert!(xml.contains("System.Management.Automation.Runspaces.PSThreadOptions"));
    }
    
    #[test]
    fn test_expected_vs_actual_format() {
        let option = PSThreadOptions::Default;
        let ps_obj = PsObject::from(option);
        let xml = ps_obj.to_element().to_string();
        
        // Expected format (ignoring RefIds for now):
        let expected = r#"<Obj><TN RefId="0"><T>System.Management.Automation.Runspaces.PSThreadOptions</T><T>System.Enum</T><T>System.ValueType</T><T>System.Object</T></TN><ToString>Default</ToString><I32>0</I32></Obj>"#;
        
        println!("Expected: {}", expected);
        println!("Actual:   {}", xml);
        
        // Now they should match exactly!
        assert_eq!(xml, expected, "PSThreadOptions XML serialization should match expected format");
    }
}