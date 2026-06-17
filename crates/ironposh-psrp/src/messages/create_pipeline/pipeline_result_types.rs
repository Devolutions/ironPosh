use ironposh_macros::PsEnum;

/// MS-PSRP PipelineResultTypes enum, serialized as a full enum `<Obj>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PsEnum)]
#[ps(
    repr = "object",
    type_names(
        "System.Management.Automation.Runspaces.PipelineResultTypes",
        "System.Enum",
        "System.ValueType",
        "System.Object"
    )
)]
pub enum PipelineResultTypes {
    #[default]
    None = 0x00,
    Output = 0x01,
    Error = 0x02,
    Warning = 0x04,
    Verbose = 0x08,
    Debug = 0x10,
    All = 0x20,
    Null = 0x40,
}

impl From<i32> for PipelineResultTypes {
    /// Lenient: a flag *combination* (e.g. `Output|Error` = 3) or unknown value
    /// maps to `None`, matching PowerShell's tolerant merge-result handling.
    fn from(value: i32) -> Self {
        match value {
            0x01 => Self::Output,
            0x02 => Self::Error,
            0x04 => Self::Warning,
            0x08 => Self::Verbose,
            0x10 => Self::Debug,
            0x20 => Self::All,
            0x40 => Self::Null,
            _ => Self::None,
        }
    }
}

/// `#[ps(with)]` converter for the merge-result fields: serializes via the enum
/// object, parses leniently (flag combos / unknown → `None`).
pub mod merge_result_conv {
    use super::PipelineResultTypes;
    use crate::PowerShellRemotingError;
    use crate::ps_value::{
        ComplexObject, ComplexObjectContent, PsEnums, PsPrimitiveValue, PsValue,
    };

    #[allow(clippy::trivially_copy_pass_by_ref)] // signature fixed by #[ps(with)]
    pub fn to_ps_value(value: &PipelineResultTypes) -> PsValue {
        PsValue::Object(ComplexObject::from(*value))
    }

    #[allow(clippy::unnecessary_wraps)] // signature fixed by #[ps(with)]
    pub fn from_ps_value(value: &PsValue) -> Result<PipelineResultTypes, PowerShellRemotingError> {
        let id = match value {
            PsValue::Object(o) => match &o.content {
                ComplexObjectContent::PsEnums(PsEnums { value }) => *value,
                ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(i)) => *i,
                _ => 0,
            },
            PsValue::Primitive(PsPrimitiveValue::I32(i)) => *i,
            PsValue::Primitive(_) => 0,
        };
        Ok(PipelineResultTypes::from(id))
    }
}
