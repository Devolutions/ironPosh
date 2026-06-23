use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize, PsUnion};

/// A `HostInformationMessage` (from `Write-Host`), macro-derived as a typed object.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(type_names("System.Management.Automation.HostInformationMessage", "System.Object"))]
pub struct HostInformationMessage {
    #[ps(name = "Message", to_string)]
    pub message: String,
    #[builder(default)]
    #[ps(name = "ForegroundColor")]
    pub foreground_color: Option<i32>,
    #[builder(default)]
    #[ps(name = "BackgroundColor")]
    pub background_color: Option<i32>,
    #[builder(default = false)]
    #[ps(name = "NoNewLine", default)]
    pub no_new_line: bool,
}

/// The `MessageData` of an INFORMATION_RECORD — an untagged polymorphic union.
///
/// Macro-derived via [`PsUnion`]: a bare string, a typed `HostInformationMessage`
/// object, or any other remote object (the dynamic escape hatch).
#[derive(Debug, Clone, PartialEq, Eq, PsUnion)]
pub enum InformationMessageData {
    #[ps(primitive)]
    String(String),
    #[ps(type_match = "HostInformationMessage")]
    HostInformationMessage(HostInformationMessage),
    #[ps(fallback)]
    Object(PsValue),
}

impl Default for InformationMessageData {
    fn default() -> Self {
        Self::String(String::new())
    }
}

impl std::fmt::Display for InformationMessageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => f.write_str(s),
            Self::HostInformationMessage(m) => f.write_str(&m.message),
            Self::Object(v) => write!(f, "{v}"),
        }
    }
}

/// INFORMATION_RECORD (MS-PSRP §2.2.2.26). Fully macro-derived.
///
/// `message_data` dispatches through [`InformationMessageData`]'s `PsUnion`;
/// `time_generated` is a `DateTime`-or-string primitive and `tags` is a
/// `System.String[]` object, each handled by a small typed converter.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(
    message_type = InformationRecord,
    type_names(
        "System.Management.Automation.InformationRecord",
        "System.Management.Automation.InformationalRecord",
        "System.Object"
    )
)]
pub struct InformationRecord {
    #[ps(name = "MessageData", default, to_string)]
    pub message_data: InformationMessageData,
    #[builder(default = false)]
    #[ps(name = "SerializeInvocationInfo", default)]
    pub serialize_invocation_info: bool,
    #[builder(default)]
    #[ps(name = "Source")]
    pub source: Option<String>,
    #[builder(default)]
    #[ps(name = "TimeGenerated", with = "datetime_conv")]
    pub time_generated: Option<String>,
    #[builder(default)]
    #[ps(name = "Tags", with = "tags_conv")]
    pub tags: Option<Vec<String>>,
    #[builder(default)]
    #[ps(name = "User")]
    pub user: Option<String>,
    #[builder(default)]
    #[ps(name = "Computer")]
    pub computer: Option<String>,
    #[builder(default)]
    #[ps(name = "ProcessId")]
    pub process_id: Option<i32>,
    #[builder(default)]
    #[ps(name = "NativeThreadId")]
    pub native_thread_id: Option<i32>,
    #[builder(default)]
    #[ps(name = "ManagedThreadId")]
    pub managed_thread_id: Option<i32>,
}

/// `#[ps(with)]`: a `DateTime`-or-`String` primitive carried as a `String`.
mod datetime_conv {
    use crate::PowerShellRemotingError;
    use crate::ps_value::{PsPrimitiveValue, PsValue};

    pub fn to_ps_value(value: &str) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::Str(value.to_string()))
    }

    #[allow(clippy::unnecessary_wraps)] // signature fixed by #[ps(with)]
    pub fn from_ps_value(value: &PsValue) -> Result<String, PowerShellRemotingError> {
        Ok(match value {
            PsValue::Primitive(PsPrimitiveValue::Str(s) | PsPrimitiveValue::DateTime(s)) => {
                s.clone()
            }
            _ => String::new(),
        })
    }
}

/// `#[ps(with)]`: a `System.String[]` whose members are keyed by index.
mod tags_conv {
    use crate::PowerShellRemotingError;
    use crate::ps_value::{ComplexObject, PsPrimitiveValue, PsValue};
    use std::borrow::Cow;

    pub fn to_ps_value(tags: &[String]) -> PsValue {
        let mut builder = ComplexObject::standard().type_names([
            Cow::Borrowed("System.String[]"),
            Cow::Borrowed("System.Array"),
            Cow::Borrowed("System.Object"),
        ]);
        for (i, tag) in tags.iter().enumerate() {
            builder = builder.extended(i.to_string(), tag.clone());
        }
        builder.build_value()
    }

    #[allow(clippy::unnecessary_wraps)] // signature fixed by #[ps(with)]
    pub fn from_ps_value(value: &PsValue) -> Result<Vec<String>, PowerShellRemotingError> {
        let mut out = Vec::new();
        if let PsValue::Object(obj) = value {
            for (_, v) in obj.properties.extended() {
                if let PsValue::Primitive(PsPrimitiveValue::Str(s)) = v {
                    out.push(s.clone());
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ps_value::{ComplexObject, PsObjectWithType};

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
