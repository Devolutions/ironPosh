use serde::{Deserialize, Serialize};

/// HostInfo (see section 2.2.3.14)
/// 
/// Represents host information containing default data and host state flags.
#[derive(Debug, Deserialize, Serialize)]
pub struct HostInfo {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub members: HostInfoMembers,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HostInfoMembers {
    #[serde(rename = "Obj", skip_serializing_if = "Option::is_none")]
    pub host_default_data: Option<HostDefaultData>,
    
    #[serde(rename = "B", default)]
    pub bool_values: Vec<BoolValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HostDefaultData {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub members: HostDefaultDataMembers,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HostDefaultDataMembers {
    #[serde(rename = "Obj", skip_serializing_if = "Option::is_none")]
    pub data: Option<DataHashtable>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataHashtable {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "TN", skip_serializing_if = "Option::is_none")]
    pub type_names: Option<TypeNames>,
    #[serde(rename = "DCT")]
    pub dictionary: Dictionary,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TypeNames {
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "T")]
    pub types: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dictionary {
    #[serde(rename = "En")]
    pub entries: Vec<DictionaryEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DictionaryEntry {
    #[serde(rename = "I32")]
    pub key: I32Value,
    
    #[serde(rename = "Obj")]
    pub value: DictionaryValue,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DictionaryValue {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub members: DictionaryValueMembers,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DictionaryValueMembers {
    #[serde(rename = "S", default)]
    pub string_values: Vec<StringValue>,
    
    #[serde(rename = "I32", skip_serializing_if = "Option::is_none")]
    pub int_value: Option<I32Value>,
    
    #[serde(rename = "Obj", skip_serializing_if = "Option::is_none")]
    pub object_value: Option<ComplexValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ComplexValue {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub members: ComplexValueMembers,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ComplexValueMembers {
    #[serde(rename = "I32", default)]
    pub int_values: Vec<I32Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I32Value {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StringValue {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BoolValue {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: bool,
}

impl HostInfo {
    /// Create a new basic HostInfo with default settings
    pub fn new_basic(ref_id: u32) -> Self {
        HostInfo {
            name: "HostInfo".to_string(),
            ref_id: Some(ref_id),
            members: HostInfoMembers {
                host_default_data: Some(HostDefaultData {
                    name: "_hostDefaultData".to_string(),
                    ref_id: Some(ref_id + 1),
                    members: HostDefaultDataMembers {
                        data: Some(DataHashtable {
                            name: "data".to_string(),
                            ref_id: Some(ref_id + 2),
                            type_names: Some(TypeNames {
                                ref_id: Some(2),
                                types: vec![
                                    "System.Collections.Hashtable".to_string(),
                                    "System.Object".to_string(),
                                ],
                            }),
                            dictionary: Dictionary {
                                entries: vec![],
                            },
                        }),
                    },
                }),
                bool_values: vec![
                    BoolValue {
                        name: "_isHostNull".to_string(),
                        value: false,
                    },
                    BoolValue {
                        name: "_isHostUINull".to_string(),
                        value: false,
                    },
                    BoolValue {
                        name: "_isHostRawUINull".to_string(),
                        value: false,
                    },
                    BoolValue {
                        name: "_useRunspaceHost".to_string(),
                        value: false,
                    },
                ],
            },
        }
    }

    /// Add a host data entry to the host default data
    pub fn add_host_data_entry(&mut self, key: i32, type_name: &str, value: HostDataValue) {
        if let Some(ref mut host_default_data) = self.members.host_default_data {
            if let Some(ref mut data) = host_default_data.members.data {
                let entry = DictionaryEntry {
                    key: I32Value {
                        name: "Key".to_string(),
                        value: key,
                    },
                    value: DictionaryValue {
                        name: "Value".to_string(),
                        ref_id: Some(data.dictionary.entries.len() as u32 + 7),
                        members: match value {
                            HostDataValue::String(s) => DictionaryValueMembers {
                                string_values: vec![
                                    StringValue {
                                        name: "T".to_string(),
                                        value: type_name.to_string(),
                                    },
                                    StringValue {
                                        name: "V".to_string(),
                                        value: s,
                                    },
                                ],
                                int_value: None,
                                object_value: None,
                            },
                            HostDataValue::Int(i) => DictionaryValueMembers {
                                string_values: vec![
                                    StringValue {
                                        name: "T".to_string(),
                                        value: type_name.to_string(),
                                    },
                                ],
                                int_value: Some(I32Value {
                                    name: "V".to_string(),
                                    value: i,
                                }),
                                object_value: None,
                            },
                            HostDataValue::Size { width, height } => DictionaryValueMembers {
                                string_values: vec![
                                    StringValue {
                                        name: "T".to_string(),
                                        value: type_name.to_string(),
                                    },
                                ],
                                int_value: None,
                                object_value: Some(ComplexValue {
                                    name: "V".to_string(),
                                    ref_id: Some(data.dictionary.entries.len() as u32 + 8),
                                    members: ComplexValueMembers {
                                        int_values: vec![
                                            I32Value {
                                                name: "width".to_string(),
                                                value: width,
                                            },
                                            I32Value {
                                                name: "height".to_string(),
                                                value: height,
                                            },
                                        ],
                                    },
                                }),
                            },
                            HostDataValue::Coordinates { x, y } => DictionaryValueMembers {
                                string_values: vec![
                                    StringValue {
                                        name: "T".to_string(),
                                        value: type_name.to_string(),
                                    },
                                ],
                                int_value: None,
                                object_value: Some(ComplexValue {
                                    name: "V".to_string(),
                                    ref_id: Some(data.dictionary.entries.len() as u32 + 8),
                                    members: ComplexValueMembers {
                                        int_values: vec![
                                            I32Value {
                                                name: "x".to_string(),
                                                value: x,
                                            },
                                            I32Value {
                                                name: "y".to_string(),
                                                value: y,
                                            },
                                        ],
                                    },
                                }),
                            },
                        },
                    },
                };
                data.dictionary.entries.push(entry);
            }
        }
    }

    /// Get the boolean value by name
    pub fn get_bool_value(&self, name: &str) -> Option<bool> {
        self.members.bool_values
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.value)
    }
}

/// Enum representing different types of host data values
#[derive(Debug, Clone)]
pub enum HostDataValue {
    String(String),
    Int(i32),
    Size { width: i32, height: i32 },
    Coordinates { x: i32, y: i32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use quick_xml::se::to_string;

    #[test]
    fn test_basic_host_info() {
        let mut host_info = HostInfo::new_basic(4);
        
        // Add some typical host data entries
        host_info.add_host_data_entry(9, "System.String", HostDataValue::String("Windows PowerShell V2 (MS Internal Only)".to_string()));
        host_info.add_host_data_entry(8, "System.Management.Automation.Host.Size", HostDataValue::Size { width: 181, height: 98 });
        host_info.add_host_data_entry(4, "System.Int32", HostDataValue::Int(25));
        host_info.add_host_data_entry(3, "System.Management.Automation.Host.Coordinates", HostDataValue::Coordinates { x: 0, y: 0 });

        assert_eq!(host_info.name, "HostInfo");
        assert_eq!(host_info.ref_id, Some(4));
        assert_eq!(host_info.get_bool_value("_isHostNull"), Some(false));
        assert_eq!(host_info.get_bool_value("_isHostUINull"), Some(false));
        assert_eq!(host_info.get_bool_value("_isHostRawUINull"), Some(false));
        assert_eq!(host_info.get_bool_value("_useRunspaceHost"), Some(false));

        // Verify we have the expected number of entries
        if let Some(ref host_default_data) = host_info.members.host_default_data {
            if let Some(ref data) = host_default_data.members.data {
                assert_eq!(data.dictionary.entries.len(), 4);
            }
        }
    }

    #[test]
    fn test_serialize_host_info() {
        let host_info = HostInfo::new_basic(4);
        
        let xml = to_string(&host_info).expect("Failed to serialize");
        println!("Serialized HostInfo XML: {}", xml);

        // Test round-trip: deserialize the serialized XML
        let deserialized: HostInfo = from_str(&xml).expect("Failed to deserialize serialized XML");
        
        assert_eq!(deserialized.name, "HostInfo");
        assert_eq!(deserialized.ref_id, Some(4));
        assert_eq!(deserialized.get_bool_value("_isHostNull"), Some(false));
    }
}
