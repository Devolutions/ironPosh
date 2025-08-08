use super::super::{ComplexObject, ComplexObjectContent, PsType, PsPrimitiveValue, PsEnums};
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PSThreadOptions {
    Default = 0,
    UseNewThread = 1,
    ReuseThread = 2,
    UseCurrentThread = 3,
}

impl From<PSThreadOptions> for ComplexObject {
    fn from(option: PSThreadOptions) -> Self {
        let type_def = PsType {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.Runspaces.PSThreadOptions"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        };
        
        let to_string = match option {
            PSThreadOptions::Default => "Default".to_string(),
            PSThreadOptions::UseNewThread => "UseNewThread".to_string(),
            PSThreadOptions::ReuseThread => "ReuseThread".to_string(),
            PSThreadOptions::UseCurrentThread => "UseCurrentThread".to_string(),
        };
        
        ComplexObject {
            type_def: Some(type_def),
            to_string: Some(to_string),
            content: ComplexObjectContent::PsEnums(PsEnums { value: option as i32 }),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        }
    }
}

// TODO: Add tests for new ComplexObject representation