use super::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
};
use crate::MessageType;
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct InformationRecord {
    pub message_data: String,
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

impl From<InformationRecord> for ComplexObject {
    fn from(record: InformationRecord) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "InformationalRecord_Message".to_string(),
            PsProperty {
                name: "InformationalRecord_Message".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(record.message_data.clone())),
            },
        );

        extended_properties.insert(
            "InformationalRecord_SerializeInvocationInfo".to_string(),
            PsProperty {
                name: "InformationalRecord_SerializeInvocationInfo".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(record.serialize_invocation_info)),
            },
        );

        if let Some(source) = record.source {
            extended_properties.insert(
                "InformationalRecord_Source".to_string(),
                PsProperty {
                    name: "InformationalRecord_Source".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(source)),
                },
            );
        }

        if let Some(time) = record.time_generated {
            extended_properties.insert(
                "InformationalRecord_TimeGenerated".to_string(),
                PsProperty {
                    name: "InformationalRecord_TimeGenerated".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(time)),
                },
            );
        }

        if let Some(tags) = record.tags
            && !tags.is_empty()
        {
            let tag_values: Vec<PsValue> = tags
                .into_iter()
                .map(|tag| PsValue::Primitive(PsPrimitiveValue::Str(tag)))
                .collect();

            // Create array-like structure for tags
            let tags_obj = ComplexObject {
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
                extended_properties: tag_values
                    .into_iter()
                    .enumerate()
                    .map(|(i, val)| {
                        (
                            i.to_string(),
                            PsProperty {
                                name: i.to_string(),
                                value: val,
                            },
                        )
                    })
                    .collect(),
            };

            extended_properties.insert(
                "InformationalRecord_Tags".to_string(),
                PsProperty {
                    name: "InformationalRecord_Tags".to_string(),
                    value: PsValue::Object(tags_obj),
                },
            );
        }

        if let Some(user) = record.user {
            extended_properties.insert(
                "InformationalRecord_User".to_string(),
                PsProperty {
                    name: "InformationalRecord_User".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(user)),
                },
            );
        }

        if let Some(computer) = record.computer {
            extended_properties.insert(
                "InformationalRecord_Computer".to_string(),
                PsProperty {
                    name: "InformationalRecord_Computer".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(computer)),
                },
            );
        }

        if let Some(pid) = record.process_id {
            extended_properties.insert(
                "InformationalRecord_ProcessId".to_string(),
                PsProperty {
                    name: "InformationalRecord_ProcessId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(pid)),
                },
            );
        }

        if let Some(native_tid) = record.native_thread_id {
            extended_properties.insert(
                "InformationalRecord_NativeThreadId".to_string(),
                PsProperty {
                    name: "InformationalRecord_NativeThreadId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(native_tid)),
                },
            );
        }

        if let Some(managed_tid) = record.managed_thread_id {
            extended_properties.insert(
                "InformationalRecord_ManagedThreadId".to_string(),
                PsProperty {
                    name: "InformationalRecord_ManagedThreadId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(managed_tid)),
                },
            );
        }

        ComplexObject {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Management.Automation.InformationRecord"),
                    Cow::Borrowed("System.Management.Automation.InformationalRecord"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: Some(record.message_data),
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for InformationRecord {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let message_data = value
            .extended_properties
            .get("InformationalRecord_Message")
            .ok_or_else(|| {
                Self::Error::InvalidMessage(
                    "Missing InformationalRecord_Message property".to_string(),
                )
            })?;
        let message_data = match &message_data.value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "InformationalRecord_Message property is not a string".to_string(),
                ));
            }
        };

        let serialize_invocation_info = value
            .extended_properties
            .get("InformationalRecord_SerializeInvocationInfo")
            .map(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Bool(b)) => *b,
                _ => false,
            })
            .unwrap_or(false);

        let source = value
            .extended_properties
            .get("InformationalRecord_Source")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let time_generated = value
            .extended_properties
            .get("InformationalRecord_TimeGenerated")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let tags = value
            .extended_properties
            .get("InformationalRecord_Tags")
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
                _ => None,
            });

        let user = value
            .extended_properties
            .get("InformationalRecord_User")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let computer = value
            .extended_properties
            .get("InformationalRecord_Computer")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let process_id = value
            .extended_properties
            .get("InformationalRecord_ProcessId")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                _ => None,
            });

        let native_thread_id = value
            .extended_properties
            .get("InformationalRecord_NativeThreadId")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                _ => None,
            });

        let managed_thread_id = value
            .extended_properties
            .get("InformationalRecord_ManagedThreadId")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                _ => None,
            });

        Ok(InformationRecord::builder()
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
            .message_data("This is an informational message.".to_string())
            .serialize_invocation_info(false)
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = InformationRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_information_record_with_metadata() {
        let record = InformationRecord::builder()
            .message_data("Test message with metadata".to_string())
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
            .message_data("Tagged message".to_string())
            .tags(Some(vec!["tag1".to_string(), "tag2".to_string()]))
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = InformationRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_message_type() {
        let record = InformationRecord::builder()
            .message_data("Test".to_string())
            .build();

        assert_eq!(record.message_type().value(), 0x00041011);
    }

    #[test]
    fn test_to_string_property() {
        let record = InformationRecord::builder()
            .message_data("Test message".to_string())
            .build();

        let complex_obj = ComplexObject::from(record);
        assert_eq!(complex_obj.to_string, Some("Test message".to_string()));
    }

    #[test]
    fn test_type_names() {
        let record = InformationRecord::builder()
            .message_data("Test".to_string())
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
