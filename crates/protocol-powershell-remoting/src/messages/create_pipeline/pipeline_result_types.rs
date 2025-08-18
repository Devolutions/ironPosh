use super::super::{ComplexObject, ComplexObjectContent, PsEnums, PsType};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineResultTypes {
    None = 0x00,
    Output = 0x01,
    Error = 0x02,
    Warning = 0x04,
    Verbose = 0x08,
    Debug = 0x10,
    All = 0x20,
    Null = 0x40,
}

impl Default for PipelineResultTypes {
    fn default() -> Self {
        PipelineResultTypes::None
    }
}

impl PipelineResultTypes {
    pub fn value(self) -> i32 {
        self as i32
    }
}

impl From<i32> for PipelineResultTypes {
    fn from(value: i32) -> Self {
        match value {
            0x00 => PipelineResultTypes::None,
            0x01 => PipelineResultTypes::Output,
            0x02 => PipelineResultTypes::Error,
            0x04 => PipelineResultTypes::Warning,
            0x08 => PipelineResultTypes::Verbose,
            0x10 => PipelineResultTypes::Debug,
            0x20 => PipelineResultTypes::All,
            0x40 => PipelineResultTypes::Null,
            _ => PipelineResultTypes::None,
        }
    }
}

impl From<PipelineResultTypes> for ComplexObject {
    fn from(result_type: PipelineResultTypes) -> Self {
        ComplexObject {
            type_def: Some(PsType::pipeline_result_types()),
            to_string: None,
            content: ComplexObjectContent::PsEnums(PsEnums {
                value: result_type.value(),
            }),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        }
    }
}

impl TryFrom<ComplexObject> for PipelineResultTypes {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, crate::PowerShellRemotingError> {
        match value.content {
            ComplexObjectContent::PsEnums(PsEnums { value: val }) => {
                Ok(PipelineResultTypes::from(val))
            }
            _ => Err(crate::PowerShellRemotingError::InvalidMessage(
                "PipelineResultTypes must be an enum".to_string(),
            )),
        }
    }
}
