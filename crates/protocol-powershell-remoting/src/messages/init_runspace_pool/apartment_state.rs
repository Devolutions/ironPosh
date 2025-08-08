use crate::{PsObject, PsProperty, PsValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApartmentState {
    STA = 0,
    MTA = 1,
    Unknown = 2,
}

impl From<ApartmentState> for PsObject {
    fn from(state: ApartmentState) -> Self {
        PsObject {
            type_names: Some(vec![
                "System.Threading.ApartmentState".to_string(),
                "System.Enum".to_string(),
                "System.ValueType".to_string(),
                "System.Object".to_string(),
            ]),
            to_string: Some(match state {
                ApartmentState::STA => "STA".to_string(),
                ApartmentState::MTA => "MTA".to_string(),
                ApartmentState::Unknown => "Unknown".to_string(),
            }),
            enum_value: Some(state as i32),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_apartment_state_serialization() {
        let state = ApartmentState::Unknown;
        let ps_obj = PsObject::from(state);
        let xml = ps_obj.to_element().to_string();
        
        println!("Generated XML:");
        println!("{}", xml);
        
        // Check basic structure
        assert!(xml.contains("<ToString>Unknown</ToString>"));
        assert!(xml.contains("<I32>2</I32>"));
        assert!(xml.contains("System.Threading.ApartmentState"));
    }
    
    #[test]
    fn test_apartment_state_expected_vs_actual_format() {
        let state = ApartmentState::Unknown;
        let ps_obj = PsObject::from(state);
        let xml = ps_obj.to_element().to_string();
        
        // Expected format (ignoring RefIds for now):
        let expected = r#"<Obj><TN RefId="0"><T>System.Threading.ApartmentState</T><T>System.Enum</T><T>System.ValueType</T><T>System.Object</T></TN><ToString>Unknown</ToString><I32>2</I32></Obj>"#;
        
        println!("Expected: {}", expected);
        println!("Actual:   {}", xml);
        
        // Now they should match exactly!
        assert_eq!(xml, expected, "ApartmentState XML serialization should match expected format");
    }
}