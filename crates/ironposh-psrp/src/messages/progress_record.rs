use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
};
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressRecordType {
    Processing = 0,
    Completed = 1,
}

impl ProgressRecordType {
    pub fn as_i32(&self) -> i32 {
        match self {
            Self::Processing => 0,
            Self::Completed => 1,
        }
    }

    pub fn as_string(&self) -> &'static str {
        match self {
            Self::Processing => "Processing",
            Self::Completed => "Completed",
        }
    }
}

impl TryFrom<i32> for ProgressRecordType {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Processing),
            1 => Ok(Self::Completed),
            _ => Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Invalid ProgressRecordType value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct ProgressRecord {
    pub activity: String,
    pub activity_id: i32,
    #[builder(default)]
    pub status_description: Option<String>,
    #[builder(default)]
    pub current_operation: Option<String>,
    #[builder(default, setter(transform = |x: Option<i32>| x.filter(|&v| v >= 0)))]
    pub parent_activity_id: Option<i32>,
    #[builder(default, setter(transform = |x: i32| if (-1..=100).contains(&x) { x } else { -1 }))]
    pub percent_complete: i32,
    #[builder(default = ProgressRecordType::Processing)]
    pub progress_type: ProgressRecordType,
    #[builder(default)]
    pub seconds_remaining: Option<i32>,
}

impl PsObjectWithType for ProgressRecord {
    fn message_type(&self) -> MessageType {
        MessageType::ProgressRecord
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<ProgressRecord> for ComplexObject {
    fn from(record: ProgressRecord) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "Activity".to_string(),
            PsProperty {
                name: "Activity".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(record.activity)),
            },
        );

        extended_properties.insert(
            "ActivityId".to_string(),
            PsProperty {
                name: "ActivityId".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(record.activity_id)),
            },
        );

        if let Some(status) = record.status_description {
            extended_properties.insert(
                "StatusDescription".to_string(),
                PsProperty {
                    name: "StatusDescription".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(status)),
                },
            );
        }

        if let Some(current_op) = record.current_operation {
            extended_properties.insert(
                "CurrentOperation".to_string(),
                PsProperty {
                    name: "CurrentOperation".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(current_op)),
                },
            );
        }

        if let Some(parent_id) = record.parent_activity_id {
            extended_properties.insert(
                "ParentActivityId".to_string(),
                PsProperty {
                    name: "ParentActivityId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(parent_id)),
                },
            );
        }

        extended_properties.insert(
            "PercentComplete".to_string(),
            PsProperty {
                name: "PercentComplete".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(record.percent_complete)),
            },
        );

        let progress_type_obj = Self {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Management.Automation.ProgressRecordType"),
                    Cow::Borrowed("System.Enum"),
                    Cow::Borrowed("System.ValueType"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: Some(record.progress_type.as_string().to_string()),
            content: ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(
                record.progress_type.as_i32(),
            )),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        };

        extended_properties.insert(
            "Type".to_string(),
            PsProperty {
                name: "Type".to_string(),
                value: PsValue::Object(progress_type_obj),
            },
        );

        if let Some(seconds) = record.seconds_remaining {
            extended_properties.insert(
                "SecondsRemaining".to_string(),
                PsProperty {
                    name: "SecondsRemaining".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(seconds)),
                },
            );
        }

        Self {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for ProgressRecord {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let activity = value
            .extended_properties
            .get("Activity")
            .ok_or_else(|| Self::Error::InvalidMessage("Missing Activity property".to_string()))?;
        let activity = match &activity.value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "Activity property is not a string".to_string(),
                ));
            }
        };

        let activity_id = value.extended_properties.get("ActivityId").ok_or_else(|| {
            Self::Error::InvalidMessage("Missing ActivityId property".to_string())
        })?;
        let activity_id = match &activity_id.value {
            PsValue::Primitive(PsPrimitiveValue::I32(id)) => *id,
            _ => {
                return Err(Self::Error::InvalidMessage(
                    "ActivityId property is not an I32".to_string(),
                ));
            }
        };

        let status_description =
            value
                .extended_properties
                .get("StatusDescription")
                .and_then(|prop| match &prop.value {
                    PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                    _ => None,
                });

        let current_operation =
            value
                .extended_properties
                .get("CurrentOperation")
                .and_then(|prop| match &prop.value {
                    PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                    _ => None,
                });

        let parent_activity_id =
            value
                .extended_properties
                .get("ParentActivityId")
                .and_then(|prop| match &prop.value {
                    PsValue::Primitive(PsPrimitiveValue::I32(id)) if *id >= 0 => Some(*id),
                    _ => None,
                });

        let percent_complete =
            value
                .extended_properties
                .get("PercentComplete")
                .map_or(-1, |prop| match &prop.value {
                    PsValue::Primitive(PsPrimitiveValue::I32(percent)) => *percent,
                    _ => -1,
                });

        let progress_type = value
            .extended_properties
            .get("Type")
            .and_then(|prop| match &prop.value {
                PsValue::Object(obj) => match &obj.content {
                    ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(val)) => {
                        ProgressRecordType::try_from(*val).ok()
                    }
                    _ => None,
                },
                PsValue::Primitive(_) => None,
            })
            .unwrap_or(ProgressRecordType::Processing);

        let seconds_remaining =
            value
                .extended_properties
                .get("SecondsRemaining")
                .and_then(|prop| match &prop.value {
                    PsValue::Primitive(PsPrimitiveValue::I32(seconds)) => Some(*seconds),
                    _ => None,
                });

        Ok(Self::builder()
            .activity(activity)
            .activity_id(activity_id)
            .status_description(status_description)
            .current_operation(current_operation)
            .parent_activity_id(parent_activity_id)
            .percent_complete(percent_complete)
            .progress_type(progress_type)
            .seconds_remaining(seconds_remaining)
            .build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_record_basic() {
        let record = ProgressRecord::builder()
            .activity("Activity Name".to_string())
            .activity_id(4)
            .status_description(Some("Good".to_string()))
            .current_operation(Some("Downloading file.txt".to_string()))
            .parent_activity_id(Some(-1)) // Will be filtered out by builder transform
            .percent_complete(20)
            .progress_type(ProgressRecordType::Processing)
            .seconds_remaining(Some(30))
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = ProgressRecord::try_from(complex_obj).unwrap();

        // Parent activity ID should be None due to builder transform filtering negative values
        let expected = ProgressRecord::builder()
            .activity("Activity Name".to_string())
            .activity_id(4)
            .status_description(Some("Good".to_string()))
            .current_operation(Some("Downloading file.txt".to_string()))
            .parent_activity_id(None)
            .percent_complete(20)
            .progress_type(ProgressRecordType::Processing)
            .seconds_remaining(Some(30))
            .build();

        assert_eq!(expected, roundtrip);
    }

    #[test]
    fn test_progress_record_completed() {
        let record = ProgressRecord::builder()
            .activity("Completed Task".to_string())
            .activity_id(1)
            .percent_complete(100)
            .progress_type(ProgressRecordType::Completed)
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = ProgressRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_message_type() {
        let record = ProgressRecord::builder()
            .activity("Test".to_string())
            .activity_id(0)
            .build();

        assert_eq!(record.message_type().value(), 0x00041010);
    }

    #[test]
    fn test_percent_complete_bounds() {
        // Test valid range
        let record = ProgressRecord::builder()
            .activity("Test".to_string())
            .activity_id(0)
            .percent_complete(50)
            .build();
        assert_eq!(record.percent_complete, 50);

        // Test out of range gets clamped to -1 by builder
        let record = ProgressRecord::builder()
            .activity("Test".to_string())
            .activity_id(0)
            .percent_complete(150) // Will be transformed to -1
            .build();
        assert_eq!(record.percent_complete, -1);
    }
}
