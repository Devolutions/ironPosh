use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
};
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct HostInformationMessage {
    pub message: String,
    #[builder(default)]
    pub foreground_color: Option<i32>,
    #[builder(default)]
    pub background_color: Option<i32>,
    #[builder(default = false)]
    pub no_new_line: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InformationMessageData {
    String(String),
    HostInformationMessage(HostInformationMessage),
    Object(PsValue),
}

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct InformationRecord {
    pub message_data: InformationMessageData,
    #[builder(default = false)]
    pub serialize_invocation_info: bool,
    #[builder(default)]
    pub source: Option<String>,
    #[builder(default)]
    pub time_generated: Option<String>,
    #[builder(default)]
    pub tags: Option<Vec<String>>,
    #[builder(default)]
    pub user: Option<String>,
    #[builder(default)]
    pub computer: Option<String>,
    #[builder(default)]
    pub process_id: Option<i32>,
    #[builder(default)]
    pub native_thread_id: Option<i32>,
    #[builder(default)]
    pub managed_thread_id: Option<i32>,
}

impl PsObjectWithType for InformationRecord {
    fn message_type(&self) -> MessageType {
        MessageType::InformationRecord
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

fn parse_console_color(value: &PsValue) -> Option<i32> {
    match value {
        PsValue::Primitive(PsPrimitiveValue::I32(v)) => Some(*v),
        PsValue::Primitive(_) => None,
        PsValue::Object(obj) => match &obj.content {
            ComplexObjectContent::PsEnums(e) => Some(e.value),
            ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(v)) => Some(*v),
            _ => None,
        },
    }
}

fn message_data_to_string(value: &InformationMessageData) -> String {
    match value {
        InformationMessageData::String(s) => s.clone(),
        InformationMessageData::HostInformationMessage(m) => m.message.clone(),
        InformationMessageData::Object(v) => v.to_string(),
    }
}

fn parse_message_data(value: PsValue) -> InformationMessageData {
    match value {
        PsValue::Primitive(PsPrimitiveValue::Str(s)) => InformationMessageData::String(s),
        PsValue::Primitive(other) => InformationMessageData::String(other.to_string()),
        PsValue::Object(obj) => {
            let is_host_information_message = obj.type_def.as_ref().is_some_and(|t| {
                t.type_names
                    .iter()
                    .any(|n| n.contains("HostInformationMessage"))
            });

            if !is_host_information_message {
                return InformationMessageData::Object(PsValue::Object(obj));
            }

            let get_prop = |name: &str| {
                obj.extended_properties
                    .get(name)
                    .map(|p| p.value.clone())
                    .or_else(|| obj.adapted_properties.get(name).map(|p| p.value.clone()))
            };

            let message = get_prop("Message")
                .and_then(|v| v.as_string())
                .or_else(|| obj.to_string.clone())
                .unwrap_or_default();

            let foreground_color =
                get_prop("ForegroundColor").and_then(|v| parse_console_color(&v));
            let background_color =
                get_prop("BackgroundColor").and_then(|v| parse_console_color(&v));
            let no_new_line = get_prop("NoNewLine").is_some_and(|v| match v {
                PsValue::Primitive(PsPrimitiveValue::Bool(b)) => b,
                _ => false,
            });

            InformationMessageData::HostInformationMessage(
                HostInformationMessage::builder()
                    .message(message)
                    .foreground_color(foreground_color)
                    .background_color(background_color)
                    .no_new_line(no_new_line)
                    .build(),
            )
        }
    }
}

impl From<InformationRecord> for ComplexObject {
    #[expect(clippy::too_many_lines)]
    fn from(record: InformationRecord) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "MessageData".to_string(),
            PsProperty {
                name: "MessageData".to_string(),
                value: match &record.message_data {
                    InformationMessageData::String(s) => {
                        PsValue::Primitive(PsPrimitiveValue::Str(s.clone()))
                    }
                    InformationMessageData::HostInformationMessage(m) => {
                        let mut props = BTreeMap::new();
                        props.insert(
                            "Message".to_string(),
                            PsProperty {
                                name: "Message".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Str(m.message.clone())),
                            },
                        );
                        if let Some(fg) = m.foreground_color {
                            props.insert(
                                "ForegroundColor".to_string(),
                                PsProperty {
                                    name: "ForegroundColor".to_string(),
                                    value: PsValue::Primitive(PsPrimitiveValue::I32(fg)),
                                },
                            );
                        }
                        if let Some(bg) = m.background_color {
                            props.insert(
                                "BackgroundColor".to_string(),
                                PsProperty {
                                    name: "BackgroundColor".to_string(),
                                    value: PsValue::Primitive(PsPrimitiveValue::I32(bg)),
                                },
                            );
                        }
                        props.insert(
                            "NoNewLine".to_string(),
                            PsProperty {
                                name: "NoNewLine".to_string(),
                                value: PsValue::Primitive(PsPrimitiveValue::Bool(m.no_new_line)),
                            },
                        );

                        PsValue::Object(Self {
                            type_def: Some(PsType {
                                type_names: vec![
                                    Cow::Borrowed(
                                        "System.Management.Automation.HostInformationMessage",
                                    ),
                                    Cow::Borrowed("System.Object"),
                                ],
                            }),
                            to_string: Some(m.message.clone()),
                            content: ComplexObjectContent::Standard,
                            adapted_properties: BTreeMap::new(),
                            extended_properties: props,
                        })
                    }
                    InformationMessageData::Object(v) => v.clone(),
                },
            },
        );

        extended_properties.insert(
            "SerializeInvocationInfo".to_string(),
            PsProperty {
                name: "SerializeInvocationInfo".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(record.serialize_invocation_info)),
            },
        );

        if let Some(source) = record.source {
            extended_properties.insert(
                "Source".to_string(),
                PsProperty {
                    name: "Source".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(source)),
                },
            );
        }

        if let Some(time) = record.time_generated {
            extended_properties.insert(
                "TimeGenerated".to_string(),
                PsProperty {
                    name: "TimeGenerated".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(time)),
                },
            );
        }

        if let Some(tags) = record.tags
            && !tags.is_empty()
        {
            // Create array-like structure for tags
            let tags_obj = Self {
                type_def: Some(PsType {
                    type_names: vec![
                        Cow::Borrowed("System.String[]"),
                        Cow::Borrowed("System.Array"),
                        Cow::Borrowed("System.Object"),
                    ],
                }),
                to_string: None,
                content: ComplexObjectContent::Standard,
                adapted_properties: BTreeMap::new(),
                extended_properties: tags
                    .into_iter()
                    .enumerate()
                    .map(|(i, tag)| {
                        let value = PsValue::Primitive(PsPrimitiveValue::Str(tag));
                        (
                            i.to_string(),
                            PsProperty {
                                name: i.to_string(),
                                value,
                            },
                        )
                    })
                    .collect(),
            };

            extended_properties.insert(
                "Tags".to_string(),
                PsProperty {
                    name: "Tags".to_string(),
                    value: PsValue::Object(tags_obj),
                },
            );
        }

        if let Some(user) = record.user {
            extended_properties.insert(
                "User".to_string(),
                PsProperty {
                    name: "User".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(user)),
                },
            );
        }

        if let Some(computer) = record.computer {
            extended_properties.insert(
                "Computer".to_string(),
                PsProperty {
                    name: "Computer".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(computer)),
                },
            );
        }

        if let Some(pid) = record.process_id {
            extended_properties.insert(
                "ProcessId".to_string(),
                PsProperty {
                    name: "ProcessId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(pid)),
                },
            );
        }

        if let Some(native_tid) = record.native_thread_id {
            extended_properties.insert(
                "NativeThreadId".to_string(),
                PsProperty {
                    name: "NativeThreadId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(native_tid)),
                },
            );
        }

        if let Some(managed_tid) = record.managed_thread_id {
            extended_properties.insert(
                "ManagedThreadId".to_string(),
                PsProperty {
                    name: "ManagedThreadId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(managed_tid)),
                },
            );
        }

        Self {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Management.Automation.InformationRecord"),
                    Cow::Borrowed("System.Management.Automation.InformationalRecord"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: Some(message_data_to_string(&record.message_data)),
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for InformationRecord {
    type Error = crate::PowerShellRemotingError;

    #[expect(clippy::too_many_lines)]
    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_prop = |names: &[&str]| {
            for name in names {
                if let Some(p) = value.extended_properties.get(*name) {
                    return Some(p);
                }
                if let Some(p) = value.adapted_properties.get(*name) {
                    return Some(p);
                }
            }
            None
        };

        // Spec: "MessageData". Back-compat: older/broken naming used "InformationalRecord_Message".
        let message_data_value = get_prop(&["MessageData", "InformationalRecord_Message"])
            .map_or_else(
                || PsValue::Primitive(PsPrimitiveValue::Str(String::new())),
                |p| p.value.clone(),
            );
        let message_data = parse_message_data(message_data_value);

        let serialize_invocation_info = get_prop(&[
            "SerializeInvocationInfo",
            "InformationalRecord_SerializeInvocationInfo",
        ])
        .is_some_and(|prop| {
            if let PsValue::Primitive(PsPrimitiveValue::Bool(b)) = prop.value {
                b
            } else {
                false
            }
        });

        let source =
            get_prop(&["Source", "InformationalRecord_Source"]).and_then(|prop| {
                match &prop.value {
                    PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                    _ => None,
                }
            });

        let time_generated = get_prop(&["TimeGenerated", "InformationalRecord_TimeGenerated"])
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s) | PsPrimitiveValue::DateTime(s)) => {
                    Some(s.clone())
                }
                _ => None,
            });

        let tags = value
            .extended_properties
            .get("Tags")
            .or_else(|| value.extended_properties.get("InformationalRecord_Tags"))
            .and_then(|prop| match &prop.value {
                PsValue::Object(obj) => {
                    let mut tags = Vec::new();
                    for prop in obj.extended_properties.values() {
                        if let PsValue::Primitive(PsPrimitiveValue::Str(s)) = &prop.value {
                            tags.push(s.clone());
                        }
                    }
                    if tags.is_empty() { None } else { Some(tags) }
                }
                PsValue::Primitive(_) => None,
            });

        let user = value
            .extended_properties
            .get("User")
            .or_else(|| value.extended_properties.get("InformationalRecord_User"))
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let computer = value
            .extended_properties
            .get("Computer")
            .or_else(|| {
                value
                    .extended_properties
                    .get("InformationalRecord_Computer")
            })
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let process_id = value
            .extended_properties
            .get("ProcessId")
            .or_else(|| {
                value
                    .extended_properties
                    .get("InformationalRecord_ProcessId")
            })
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                PsValue::Primitive(PsPrimitiveValue::U32(id)) => Some((*id) as i32),
                _ => None,
            });

        let native_thread_id = value
            .extended_properties
            .get("NativeThreadId")
            .or_else(|| {
                value
                    .extended_properties
                    .get("InformationalRecord_NativeThreadId")
            })
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                PsValue::Primitive(PsPrimitiveValue::U32(id)) => Some((*id) as i32),
                _ => None,
            });

        let managed_thread_id = value
            .extended_properties
            .get("ManagedThreadId")
            .or_else(|| {
                value
                    .extended_properties
                    .get("InformationalRecord_ManagedThreadId")
            })
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                PsValue::Primitive(PsPrimitiveValue::U32(id)) => Some((*id) as i32),
                _ => None,
            });

        Ok(Self::builder()
            .message_data(message_data)
            .serialize_invocation_info(serialize_invocation_info)
            .source(source)
            .time_generated(time_generated)
            .tags(tags)
            .user(user)
            .computer(computer)
            .process_id(process_id)
            .native_thread_id(native_thread_id)
            .managed_thread_id(managed_thread_id)
            .build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_information_record_basic() {
        let record = InformationRecord::builder()
            .message_data(InformationMessageData::String(
                "This is an informational message.".to_string(),
            ))
            .serialize_invocation_info(false)
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = InformationRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_information_record_with_metadata() {
        let record = InformationRecord::builder()
            .message_data(InformationMessageData::String(
                "Test message with metadata".to_string(),
            ))
            .serialize_invocation_info(true)
            .source(Some("Write-Information".to_string()))
            .user(Some("TestUser".to_string()))
            .computer(Some("TestComputer".to_string()))
            .process_id(Some(1234))
            .native_thread_id(Some(5678))
            .managed_thread_id(Some(9012))
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = InformationRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_information_record_with_tags() {
        let record = InformationRecord::builder()
            .message_data(InformationMessageData::String("Tagged message".to_string()))
            .tags(Some(vec!["tag1".to_string(), "tag2".to_string()]))
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = InformationRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_message_type() {
        let record = InformationRecord::builder()
            .message_data(InformationMessageData::String("Test".to_string()))
            .build();

        assert_eq!(record.message_type().value(), 0x00041011);
    }

    #[test]
    fn test_to_string_property() {
        let record = InformationRecord::builder()
            .message_data(InformationMessageData::String("Test message".to_string()))
            .build();

        let complex_obj = ComplexObject::from(record);
        assert_eq!(complex_obj.to_string, Some("Test message".to_string()));
    }

    #[test]
    fn test_type_names() {
        let record = InformationRecord::builder()
            .message_data(InformationMessageData::String("Test".to_string()))
            .build();

        let complex_obj = ComplexObject::from(record);
        let type_def = complex_obj.type_def.unwrap();
        assert_eq!(type_def.type_names.len(), 3);
        assert_eq!(
            type_def.type_names[0].as_ref(),
            "System.Management.Automation.InformationRecord"
        );
        assert_eq!(
            type_def.type_names[1].as_ref(),
            "System.Management.Automation.InformationalRecord"
        );
        assert_eq!(type_def.type_names[2].as_ref(), "System.Object");
    }
}
