use serde::{Deserialize, Serialize};

pub mod apartment_state;
pub mod host_info;
pub mod ps_thread_options;

pub use apartment_state::ApartmentState;
pub use host_info::HostInfo;
pub use ps_thread_options::PSThreadOptions;

/// INIT_RUNSPACEPOOL Message (MessageType: 0x00010004)
/// 
/// The Data field contains UTF-8 encoded XML representing a Complex Object
/// with extended properties for initializing a RunspacePool.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct InitRunspacepool {
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub members: MemberSet,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MemberSet {
    /// I32 values for MinRunspaces and MaxRunspaces
    #[serde(rename = "I32")]
    pub i32_values: Vec<I32Value>,

    /// Object values for PSThreadOptions, ApartmentState, and optionally HostInfo
    #[serde(rename = "Obj")]
    pub obj_values: Vec<GenericObj>,

    /// Application arguments (usually null)
    #[serde(rename = "Nil", skip_serializing_if = "Option::is_none")]
    pub application_arguments: Option<NilValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenericObj {
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
    #[serde(rename = "MS", skip_serializing_if = "Option::is_none")]
    pub member_set: Option<HostInfoMemberSet>,
    #[serde(rename = "DCT", skip_serializing_if = "Option::is_none")]
    pub dictionary: Option<Dictionary>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HostInfoMemberSet {
    #[serde(rename = "Obj", skip_serializing_if = "Option::is_none")]
    pub host_default_data: Option<HostDefaultData>,
    #[serde(rename = "B")]
    pub bool_values: Vec<BoolValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HostDefaultData {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS", skip_serializing_if = "Option::is_none")]
    pub member_set: Option<DataMemberSet>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataMemberSet {
    #[serde(rename = "Obj", skip_serializing_if = "Option::is_none")]
    pub data: Option<DataObj>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataObj {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "TN", skip_serializing_if = "Option::is_none")]
    pub type_names: Option<TypeNames>,
    #[serde(rename = "DCT", skip_serializing_if = "Option::is_none")]
    pub dictionary: Option<Dictionary>,
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
    pub value: ValueObj,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ValueObj {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub member_set: ValueMemberSet,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ValueMemberSet {
    #[serde(rename = "S")]
    pub string_values: Vec<StringValue>,
    #[serde(rename = "Obj", skip_serializing_if = "Option::is_none")]
    pub obj_value: Option<NestedObj>,
    #[serde(rename = "I32", skip_serializing_if = "Option::is_none")]
    pub int_value: Option<I32Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NestedObj {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub member_set: NestedMemberSet,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NestedMemberSet {
    #[serde(rename = "I32")]
    pub int_values: Vec<I32Value>,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct TypeNames {
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "T")]
    pub types: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I32Value {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NilValue {
    #[serde(rename = "@N")]
    pub name: String,
}

impl InitRunspacepool {
    pub fn builder() -> InitRunspacepoolBuilder {
        InitRunspacepoolBuilder::new()
    }

    pub fn min_runspaces(&self) -> i32 {
        self.members.i32_values
            .iter()
            .find(|v| v.name == "MinRunspaces")
            .map(|v| v.value)
            .unwrap_or(1)
    }

    pub fn max_runspaces(&self) -> i32 {
        self.members.i32_values
            .iter()
            .find(|v| v.name == "MaxRunspaces")
            .map(|v| v.value)
            .unwrap_or(1)
    }

    pub fn ps_thread_options(&self) -> Option<&GenericObj> {
        self.members.obj_values
            .iter()
            .find(|obj| obj.name == "PSThreadOptions")
    }

    pub fn apartment_state(&self) -> Option<&GenericObj> {
        self.members.obj_values
            .iter()
            .find(|obj| obj.name == "ApartmentState")
    }

    pub fn host_info(&self) -> Option<&GenericObj> {
        self.members.obj_values
            .iter()
            .find(|obj| obj.name == "HostInfo")
    }

    pub fn application_arguments(&self) -> Option<&NilValue> {
        self.members.application_arguments.as_ref()
    }
}

pub struct InitRunspacepoolBuilder {
    min_runspaces: i32,
    max_runspaces: i32,
    ps_thread_options: PSThreadOptions,
    apartment_state: ApartmentState,
    host_info: Option<HostInfo>,
    application_arguments: Option<NilValue>,
}

impl InitRunspacepoolBuilder {
    pub fn new() -> Self {
        Self {
            min_runspaces: 1,
            max_runspaces: 1,
            ps_thread_options: PSThreadOptions::new(2, 0, "Default"),
            apartment_state: ApartmentState::mta(3),
            host_info: None,
            application_arguments: None,
        }
    }

    pub fn min_runspaces(mut self, value: i32) -> Self {
        self.min_runspaces = value;
        self
    }

    pub fn max_runspaces(mut self, value: i32) -> Self {
        self.max_runspaces = value;
        self
    }

    pub fn ps_thread_options(mut self, value: PSThreadOptions) -> Self {
        self.ps_thread_options = value;
        self
    }

    pub fn apartment_state(mut self, value: ApartmentState) -> Self {
        self.apartment_state = value;
        self
    }

    pub fn host_info(mut self, value: HostInfo) -> Self {
        self.host_info = Some(value);
        self
    }

    pub fn application_arguments(mut self, value: NilValue) -> Self {
        self.application_arguments = Some(value);
        self
    }

    pub fn build(self) -> InitRunspacepool {
        let mut obj_values = vec![
            GenericObj {
                name: "PSThreadOptions".to_string(),
                ref_id: Some(2),
                type_names: Some(TypeNames {
                    ref_id: Some(0),
                    types: vec![
                        "System.Management.Automation.Runspaces.PSThreadOptions".to_string(),
                        "System.Enum".to_string(),
                        "System.ValueType".to_string(),
                        "System.Object".to_string(),
                    ],
                }),
                to_string: Some("Default".to_string()),
                int_value: Some(0),
                member_set: None,
                dictionary: None,
            },
            GenericObj {
                name: "ApartmentState".to_string(),
                ref_id: Some(3),
                type_names: Some(TypeNames {
                    ref_id: Some(1),
                    types: vec![
                        "System.Threading.ApartmentState".to_string(),
                        "System.Enum".to_string(),
                        "System.ValueType".to_string(),
                        "System.Object".to_string(),
                    ],
                }),
                to_string: Some("MTA".to_string()),
                int_value: Some(1),
                member_set: None,
                dictionary: None,
            },
        ];

        if self.host_info.is_some() {
            obj_values.push(GenericObj {
                name: "HostInfo".to_string(),
                ref_id: Some(4),
                type_names: None,
                to_string: None,
                int_value: None,
                member_set: Some(HostInfoMemberSet {
                    host_default_data: Some(HostDefaultData {
                        name: "_hostDefaultData".to_string(),
                        ref_id: Some(5),
                        member_set: Some(DataMemberSet {
                            data: Some(DataObj {
                                name: "data".to_string(),
                                ref_id: Some(6),
                                type_names: Some(TypeNames {
                                    ref_id: Some(2),
                                    types: vec![
                                        "System.Collections.Hashtable".to_string(),
                                        "System.Object".to_string(),
                                    ],
                                }),
                                dictionary: Some(Dictionary {
                                    entries: vec![],
                                }),
                            }),
                        }),
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
                }),
                dictionary: None,
            });
        }

        InitRunspacepool {
            ref_id: None,
            members: MemberSet {
                i32_values: vec![
                    I32Value {
                        name: "MinRunspaces".to_string(),
                        value: self.min_runspaces,
                    },
                    I32Value {
                        name: "MaxRunspaces".to_string(),
                        value: self.max_runspaces,
                    },
                ],
                obj_values,
                application_arguments: self.application_arguments,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_runspacepool_builder() {
        let host_info = HostInfo::new_basic(4);

        let ps_thread_options = PSThreadOptions::new(2, 0, "UseCurrentThread");

        let apartment_state = ApartmentState::sta(3);

        let init_runspacepool = InitRunspacepool::builder()
            .min_runspaces(1)
            .max_runspaces(5)
            .ps_thread_options(ps_thread_options)
            .apartment_state(apartment_state)
            .host_info(host_info)
            .application_arguments(NilValue {
                name: "ApplicationArguments".to_string(),
            })
            .build();

        assert_eq!(init_runspacepool.min_runspaces(), 1);
        assert_eq!(init_runspacepool.max_runspaces(), 5);
        assert!(init_runspacepool.host_info().is_some());
        assert!(init_runspacepool.application_arguments().is_some());
    }

    #[test]
    fn test_init_runspacepool_full_xml_roundtrip() {
        let xml = r#"<Obj RefId="1">
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
</Obj>"#;

        // Test deserialization
        let deserialized: InitRunspacepool = quick_xml::de::from_str(xml).expect("Failed to deserialize");

        // Verify structure
        assert_eq!(deserialized.min_runspaces(), 1);
        assert_eq!(deserialized.max_runspaces(), 1);
        assert!(deserialized.ps_thread_options().is_some());
        assert!(deserialized.apartment_state().is_some());
        assert!(deserialized.host_info().is_some());
        assert!(deserialized.application_arguments().is_some());

        // Test round-trip: serialize and deserialize again
        let serialized_xml = quick_xml::se::to_string(&deserialized).expect("Failed to serialize");
        println!("Round-trip serialized XML: {}", serialized_xml);
        
        let round_trip: InitRunspacepool = quick_xml::de::from_str(&serialized_xml).expect("Failed to deserialize round-trip");
        
        // Verify round-trip worked
        assert_eq!(round_trip.min_runspaces(), 1);
        assert_eq!(round_trip.max_runspaces(), 1);
    }
}