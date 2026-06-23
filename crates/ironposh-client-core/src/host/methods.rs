use ironposh_macros::{PsDeserialize, PsSerialize};
use ironposh_psrp::PsValue;

// Strongly-typed host data structures (MS-PSRP §2.2.3). All CLIXML conversions
// are macro-derived; host objects are read under either camelCase or PascalCase
// (`#[ps(also)]`) and serialized under both, matching PowerShell.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(type_names(
    "System.Management.Automation.Host.Coordinates",
    "System.ValueType",
    "System.Object"
))]
pub struct Coordinates {
    #[ps(name = "x", also = "X")]
    pub x: i32,
    #[ps(name = "y", also = "Y")]
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(type_names(
    "System.Management.Automation.Host.Size",
    "System.ValueType",
    "System.Object"
))]
pub struct Size {
    #[ps(name = "width", also = "Width")]
    pub width: i32,
    #[ps(name = "height", also = "Height")]
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(type_names(
    "System.Management.Automation.Host.Rectangle",
    "System.ValueType",
    "System.Object"
))]
pub struct Rectangle {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(type_names(
    "System.Management.Automation.Host.BufferCell",
    "System.ValueType",
    "System.Object"
))]
pub struct BufferCell {
    pub character: char,
    #[ps(name = "foregroundColor")]
    pub foreground: i32, // Color enum underlying int
    #[ps(name = "backgroundColor")]
    pub background: i32, // Color enum underlying int
    #[ps(name = "bufferCellType")]
    pub flags: i32, // BufferCellType, underlying int
}

#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(type_names(
    "System.Management.Automation.Host.KeyInfo",
    "System.ValueType",
    "System.Object"
))]
pub struct KeyInfo {
    #[ps(name = "virtualKeyCode", also = "VirtualKeyCode")]
    pub virtual_key_code: i32,
    #[ps(name = "character", also = "Character")]
    pub character: char,
    #[ps(name = "controlKeyState", also = "ControlKeyState")]
    pub control_key_state: i32,
    #[ps(name = "keyDown", also = "KeyDown")]
    pub key_down: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
pub struct ProgressRecord {
    #[ps(name = "Activity", default)]
    pub activity: String,
    #[ps(name = "StatusDescription", default)]
    pub status_description: String,
    #[ps(name = "CurrentOperation", default)]
    pub current_operation: String,
    #[ps(name = "ActivityId", default)]
    pub activity_id: i32,
    #[ps(name = "ParentActivityId", default)]
    pub parent_activity_id: i32,
    #[ps(name = "PercentComplete", default)]
    pub percent_complete: i32,
    #[ps(name = "SecondsRemaining", default)]
    pub seconds_remaining: i32,
    #[ps(name = "Type", default, with = "progress_type_conv")]
    pub record_type: i32, // ProgressRecordType (nested enum object)
}

#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
pub struct FieldDescription {
    #[ps(name = "name", also = "Name", default)]
    pub name: String,
    #[ps(name = "label", also = "Label", default)]
    pub label: String,
    #[ps(name = "helpMessage", also = "HelpMessage", default)]
    pub help_message: String,
    #[ps(name = "isMandatory", also = "IsMandatory", default)]
    pub is_mandatory: bool,
    #[ps(
        name = "parameterType",
        also = "ParameterType",
        also = "parameterTypeName",
        also = "ParameterTypeName",
        default
    )]
    pub parameter_type: String,
    #[ps(name = "defaultValue", also = "DefaultValue")]
    pub default_value: Option<PsValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
pub struct ChoiceDescription {
    #[ps(name = "label", also = "Label", default)]
    pub label: String,
    #[ps(name = "helpMessage", also = "HelpMessage", default)]
    pub help_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(type_names("System.Management.Automation.PSCredential", "System.Object"))]
pub struct PSCredential {
    #[ps(name = "userName", also = "UserName")]
    pub user_name: String,
    #[ps(name = "password", also = "Password", with = "secure_string_conv")]
    pub password: Vec<u8>, // SecureString as bytes
}

/// `#[ps(with)]`: a SecureString blob carried as `<SS>` (not a plain byte array).
mod secure_string_conv {
    use ironposh_psrp::PowerShellRemotingError;
    use ironposh_psrp::ps_value::{PsPrimitiveValue, PsValue};

    pub fn to_ps_value(value: &[u8]) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::SecureString(value.to_vec()))
    }

    pub fn from_ps_value(value: &PsValue) -> Result<Vec<u8>, PowerShellRemotingError> {
        match value {
            PsValue::Primitive(PsPrimitiveValue::SecureString(b) | PsPrimitiveValue::Bytes(b)) => {
                Ok(b.clone())
            }
            other => Err(PowerShellRemotingError::InvalidMessage(format!(
                "expected SecureString, got {other:?}"
            ))),
        }
    }
}

/// `#[ps(with)]`: the WriteProgress `Type` field is a nested ProgressRecordType
/// enum object; we only carry its underlying i32 (the record is parse-only).
mod progress_type_conv {
    use ironposh_psrp::PowerShellRemotingError;
    use ironposh_psrp::ps_value::{ComplexObjectContent, PsPrimitiveValue, PsValue};

    #[allow(clippy::trivially_copy_pass_by_ref)] // signature fixed by #[ps(with)]
    pub fn to_ps_value(value: &i32) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::I32(*value))
    }

    #[allow(clippy::unnecessary_wraps)] // signature fixed by #[ps(with)]
    pub fn from_ps_value(value: &PsValue) -> Result<i32, PowerShellRemotingError> {
        Ok(match value {
            PsValue::Object(obj) => match &obj.content {
                ComplexObjectContent::PsEnums(e) => e.value,
                ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(i)) => *i,
                _ => 0,
            },
            PsValue::Primitive(PsPrimitiveValue::I32(i)) => *i,
            PsValue::Primitive(_) => 0,
        })
    }
}
