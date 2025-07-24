use serde::{Deserialize, Serialize};

/// ApartmentState (see section 2.2.3.7)
/// 
/// Apartment state provided by the higher layer; PSRP MUST NOT interpret this data.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApartmentState {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "TN", skip_serializing_if = "Option::is_none")]
    pub type_names: Option<TypeNames>,
    #[serde(rename = "ToString", skip_serializing_if = "Option::is_none")]
    pub to_string: Option<String>,
    #[serde(rename = "I32", skip_serializing_if = "Option::is_none")]
    pub int_value: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TypeNames {
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "T")]
    pub types: Vec<String>,
}

impl ApartmentState {
    /// Create a new ApartmentState
    pub fn new(ref_id: u32, value: i32, string_representation: &str) -> Self {
        ApartmentState {
            name: "ApartmentState".to_string(),
            ref_id: Some(ref_id),
            type_names: Some(TypeNames {
                ref_id: Some(1),
                types: vec![
                    "System.Threading.ApartmentState".to_string(),
                    "System.Enum".to_string(),
                    "System.ValueType".to_string(),
                    "System.Object".to_string(),
                ],
            }),
            to_string: Some(string_representation.to_string()),
            int_value: Some(value),
        }
    }

    /// Create STA (Single Threaded Apartment) state (value = 0)
    pub fn sta(ref_id: u32) -> Self {
        Self::new(ref_id, 0, "STA")
    }

    /// Create MTA (Multi Threaded Apartment) state (value = 1)
    pub fn mta(ref_id: u32) -> Self {
        Self::new(ref_id, 1, "MTA")
    }

    /// Create Unknown apartment state (value = 2)
    pub fn unknown(ref_id: u32) -> Self {
        Self::new(ref_id, 2, "Unknown")
    }

    /// Get the integer value
    pub fn value(&self) -> Option<i32> {
        self.int_value
    }

    /// Get the string representation
    pub fn string_value(&self) -> Option<&str> {
        self.to_string.as_deref()
    }

    /// Check if this is STA
    pub fn is_sta(&self) -> bool {
        self.int_value == Some(0)
    }

    /// Check if this is MTA
    pub fn is_mta(&self) -> bool {
        self.int_value == Some(1)
    }

    /// Check if this is Unknown
    pub fn is_unknown(&self) -> bool {
        self.int_value == Some(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use quick_xml::se::to_string;

    #[test]
    fn test_mta_apartment_state() {
        let state = ApartmentState::mta(3);
        
        assert_eq!(state.name, "ApartmentState");
        assert_eq!(state.ref_id, Some(3));
        assert_eq!(state.value(), Some(1));
        assert_eq!(state.string_value(), Some("MTA"));
        assert!(state.is_mta());
        assert!(!state.is_sta());
        assert!(!state.is_unknown());
        
        // Verify type names
        let type_names = state.type_names.as_ref().unwrap();
        assert_eq!(type_names.ref_id, Some(1));
        assert_eq!(type_names.types.len(), 4);
        assert_eq!(type_names.types[0], "System.Threading.ApartmentState");
    }

    #[test]
    fn test_sta_apartment_state() {
        let state = ApartmentState::sta(4);
        
        assert_eq!(state.value(), Some(0));
        assert_eq!(state.string_value(), Some("STA"));
        assert!(state.is_sta());
        assert!(!state.is_mta());
        assert!(!state.is_unknown());
    }

    #[test]
    fn test_unknown_apartment_state() {
        let state = ApartmentState::unknown(5);
        
        assert_eq!(state.value(), Some(2));
        assert_eq!(state.string_value(), Some("Unknown"));
        assert!(!state.is_sta());
        assert!(!state.is_mta());
        assert!(state.is_unknown());
    }

    #[test]
    fn test_serialize_apartment_state() {
        let state = ApartmentState::mta(3);
        
        let xml = to_string(&state).expect("Failed to serialize");
        println!("Serialized ApartmentState XML: {}", xml);

        // Test round-trip: deserialize the serialized XML
        let deserialized: ApartmentState = from_str(&xml).expect("Failed to deserialize serialized XML");
        
        assert_eq!(deserialized.name, "ApartmentState");
        assert_eq!(deserialized.ref_id, Some(3));
        assert_eq!(deserialized.value(), Some(1));
        assert_eq!(deserialized.string_value(), Some("MTA"));
        assert!(deserialized.is_mta());
    }

    #[test]
    fn test_deserialize_from_example() {
        let xml = r#"
<ApartmentState N="ApartmentState" RefId="3">
  <TN RefId="1">
    <T>System.Threading.ApartmentState</T>
    <T>System.Enum</T>
    <T>System.ValueType</T>
    <T>System.Object</T>
  </TN>
  <ToString>MTA</ToString>
  <I32>1</I32>
</ApartmentState>
"#;

        let state: ApartmentState = from_str(xml).expect("Deserialization failed");
        
        assert_eq!(state.name, "ApartmentState");
        assert_eq!(state.ref_id, Some(3));
        assert_eq!(state.value(), Some(1));
        assert_eq!(state.string_value(), Some("MTA"));
        assert!(state.is_mta());
    }
}
