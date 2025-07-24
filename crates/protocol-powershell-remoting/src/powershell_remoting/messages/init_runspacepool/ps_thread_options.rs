use serde::{Deserialize, Serialize};

/// PSThreadOptions (see section 2.2.3.6)
/// 
/// Thread options provided by the higher layer; PSRP MUST NOT interpret this data.
#[derive(Debug, Deserialize, Serialize)]
pub struct PSThreadOptions {
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

impl PSThreadOptions {
    /// Create a new PSThreadOptions
    pub fn new(ref_id: u32, value: i32, string_representation: &str) -> Self {
        PSThreadOptions {
            name: "PSThreadOptions".to_string(),
            ref_id: Some(ref_id),
            type_names: Some(TypeNames {
                ref_id: Some(0),
                types: vec![
                    "System.Management.Automation.Runspaces.PSThreadOptions".to_string(),
                    "System.Enum".to_string(),
                    "System.ValueType".to_string(),
                    "System.Object".to_string(),
                ],
            }),
            to_string: Some(string_representation.to_string()),
            int_value: Some(value),
        }
    }

    /// Create Default PSThreadOptions (value = 0)
    pub fn default_options(ref_id: u32) -> Self {
        Self::new(ref_id, 0, "Default")
    }

    /// Create ReuseThread PSThreadOptions (value = 1)
    pub fn reuse_thread(ref_id: u32) -> Self {
        Self::new(ref_id, 1, "ReuseThread")
    }

    /// Create UseNewThread PSThreadOptions (value = 2)  
    pub fn use_new_thread(ref_id: u32) -> Self {
        Self::new(ref_id, 2, "UseNewThread")
    }

    /// Get the integer value
    pub fn value(&self) -> Option<i32> {
        self.int_value
    }

    /// Get the string representation
    pub fn string_value(&self) -> Option<&str> {
        self.to_string.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use quick_xml::se::to_string;

    #[test]
    fn test_default_ps_thread_options() {
        let options = PSThreadOptions::default_options(2);
        
        assert_eq!(options.name, "PSThreadOptions");
        assert_eq!(options.ref_id, Some(2));
        assert_eq!(options.value(), Some(0));
        assert_eq!(options.string_value(), Some("Default"));
        
        // Verify type names
        let type_names = options.type_names.as_ref().unwrap();
        assert_eq!(type_names.ref_id, Some(0));
        assert_eq!(type_names.types.len(), 4);
        assert_eq!(type_names.types[0], "System.Management.Automation.Runspaces.PSThreadOptions");
    }

    #[test]
    fn test_reuse_thread_options() {
        let options = PSThreadOptions::reuse_thread(3);
        
        assert_eq!(options.value(), Some(1));
        assert_eq!(options.string_value(), Some("ReuseThread"));
    }

    #[test]
    fn test_serialize_ps_thread_options() {
        let options = PSThreadOptions::default_options(2);
        
        let xml = to_string(&options).expect("Failed to serialize");
        println!("Serialized PSThreadOptions XML: {}", xml);

        // Test round-trip: deserialize the serialized XML
        let deserialized: PSThreadOptions = from_str(&xml).expect("Failed to deserialize serialized XML");
        
        assert_eq!(deserialized.name, "PSThreadOptions");
        assert_eq!(deserialized.ref_id, Some(2));
        assert_eq!(deserialized.value(), Some(0));
        assert_eq!(deserialized.string_value(), Some("Default"));
    }

    #[test]
    fn test_deserialize_from_example() {
        let xml = r#"
<PSThreadOptions N="PSThreadOptions" RefId="2">
  <TN RefId="0">
    <T>System.Management.Automation.Runspaces.PSThreadOptions</T>
    <T>System.Enum</T>
    <T>System.ValueType</T>
    <T>System.Object</T>
  </TN>
  <ToString>Default</ToString>
  <I32>0</I32>
</PSThreadOptions>
"#;

        let options: PSThreadOptions = from_str(xml).expect("Deserialization failed");
        
        assert_eq!(options.name, "PSThreadOptions");
        assert_eq!(options.ref_id, Some(2));
        assert_eq!(options.value(), Some(0));
        assert_eq!(options.string_value(), Some("Default"));
    }
}
