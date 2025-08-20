use super::super::{ComplexObject, ComplexObjectContent, PsEnums, PsType};
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApartmentState {
    STA = 0,
    MTA = 1,
    Unknown = 2,
}

impl From<ApartmentState> for ComplexObject {
    fn from(state: ApartmentState) -> Self {
        let type_def = PsType {
            type_names: vec![
                Cow::Borrowed("System.Threading.ApartmentState"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        };

        let to_string = match state {
            ApartmentState::STA => "STA".to_string(),
            ApartmentState::MTA => "MTA".to_string(),
            ApartmentState::Unknown => "Unknown".to_string(),
        };

        ComplexObject {
            type_def: Some(type_def),
            to_string: Some(to_string),
            content: ComplexObjectContent::PsEnums(PsEnums {
                value: state as i32,
            }),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        }
    }
}

// TODO: Add tests for new ComplexObject representation
