use crate::{MessageType, PSMessage, PsObject, PsProperty, PsValue};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PSThreadOptions {
    Default = 0,
    UseNewThread = 1,
    ReuseThread = 2,
    UseCurrentThread = 3,
}

impl PSMessage for InitRunspacePool {
    fn message_type(&self) -> MessageType {
        MessageType::InitRunspacepool
    }
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
            ms: vec![
                PsProperty {
                    name: None,
                    ref_id: None,
                    value: PsValue::Str(match option {
                        PSThreadOptions::Default => "Default".to_string(),
                        PSThreadOptions::UseNewThread => "UseNewThread".to_string(),
                        PSThreadOptions::ReuseThread => "ReuseThread".to_string(),
                        PSThreadOptions::UseCurrentThread => "UseCurrentThread".to_string(),
                    }),
                },
                PsProperty {
                    name: None,
                    ref_id: None,
                    value: PsValue::I32(option as i32),
                },
            ],
            ..Default::default()
        }
    }
}

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
            ms: vec![
                PsProperty {
                    name: None,
                    ref_id: None,
                    value: PsValue::Str(match state {
                        ApartmentState::STA => "STA".to_string(),
                        ApartmentState::MTA => "MTA".to_string(),
                        ApartmentState::Unknown => "Unknown".to_string(),
                    }),
                },
                PsProperty {
                    name: None,
                    ref_id: None,
                    value: PsValue::I32(state as i32),
                },
            ],
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub struct HostInfo {
    pub is_host_null: bool,
    pub is_host_ui_null: bool,
    pub is_host_raw_ui_null: bool,
    pub use_runspace_host: bool,
    pub host_default_data: HashMap<i32, PsValue>,
}


impl From<HostInfo> for PsObject {
    fn from(host_info: HostInfo) -> Self {
        let mut ms = vec![
            PsProperty {
                name: Some("_isHostNull".to_string()),
                ref_id: None,
                value: PsValue::Bool(host_info.is_host_null),
            },
            PsProperty {
                name: Some("_isHostUINull".to_string()),
                ref_id: None,
                value: PsValue::Bool(host_info.is_host_ui_null),
            },
            PsProperty {
                name: Some("_isHostRawUINull".to_string()),
                ref_id: None,
                value: PsValue::Bool(host_info.is_host_raw_ui_null),
            },
            PsProperty {
                name: Some("_useRunspaceHost".to_string()),
                ref_id: None,
                value: PsValue::Bool(host_info.use_runspace_host),
            },
        ];

        if !host_info.host_default_data.is_empty() {
            let host_data_obj = PsObject {
                ms: vec![PsProperty {
                    name: Some("data".to_string()),
                    ref_id: None,
                    value: PsValue::Object(PsObject {
                        type_names: Some(vec![
                            "System.Collections.Hashtable".to_string(),
                            "System.Object".to_string(),
                        ]),
                        dct: host_info
                            .host_default_data
                            .into_iter()
                            .map(|(k, v)| (PsValue::I32(k), v))
                            .collect(),
                        ..Default::default()
                    }),
                }],
                ..Default::default()
            };

            ms.push(PsProperty {
                name: Some("_hostDefaultData".to_string()),
                ref_id: None,
                value: PsValue::Object(host_data_obj),
            });
        }

        PsObject {
            ms,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRunspacePool {
    pub min_runspaces: i32,
    pub max_runspaces: i32,
    pub thread_options: PSThreadOptions,
    pub apartment_state: ApartmentState,
    pub host_info: Option<HostInfo>,
    pub application_arguments: HashMap<PsValue, PsValue>,
}

impl Default for InitRunspacePool {
    fn default() -> Self {
        Self {
            min_runspaces: 1,
            max_runspaces: 1,
            thread_options: PSThreadOptions::Default,
            apartment_state: ApartmentState::MTA,
            host_info: None,
            application_arguments: HashMap::new(),
        }
    }
}

impl From<InitRunspacePool> for PsObject {
    fn from(init: InitRunspacePool) -> Self {
        let mut ms = Vec::new();

        ms.push(PsProperty {
            name: Some("MinRunspaces".to_string()),
            ref_id: None,
            value: PsValue::I32(init.min_runspaces),
        });

        ms.push(PsProperty {
            name: Some("MaxRunspaces".to_string()),
            ref_id: None,
            value: PsValue::I32(init.max_runspaces),
        });

        ms.push(PsProperty {
            name: Some("PSThreadOptions".to_string()),
            ref_id: None,
            value: PsValue::Object(init.thread_options.into()),
        });

        ms.push(PsProperty {
            name: Some("ApartmentState".to_string()),
            ref_id: None,
            value: PsValue::Object(init.apartment_state.into()),
        });

        if let Some(host_info) = init.host_info.as_ref() { ms.push(PsProperty {
                name: Some("HostInfo".to_string()),
                ref_id: None,
                value: PsValue::Object(host_info.clone().into()),
            }); }

        if init.application_arguments.is_empty() {
            ms.push(PsProperty {
                name: Some("ApplicationArguments".to_string()),
                ref_id: None,
                value: PsValue::Nil,
            });
        } else {
            let app_args_obj = PsObject {
                type_names: Some(vec![
                    "System.Management.Automation.PSPrimitiveDictionary".to_string(),
                    "System.Collections.Hashtable".to_string(),
                    "System.Object".to_string(),
                ]),
                dct: init.application_arguments,
                ..Default::default()
            };
            ms.push(PsProperty {
                name: Some("ApplicationArguments".to_string()),
                ref_id: None,
                value: PsValue::Object(app_args_obj),
            });
        }

        PsObject {
            ms,
            ..Default::default()
        }
    }
}


