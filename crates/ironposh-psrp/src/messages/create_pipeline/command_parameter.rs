use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize};

/// A single pipeline command parameter: a name (`N`, `Nil` when positional)
/// and an arbitrary value (`V`, the dynamic escape hatch).
#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
pub struct CommandParameter {
    #[ps(name = "N", nil_when_none)]
    name: Option<String>,
    #[ps(name = "V")]
    value: PsValue,
}

impl CommandParameter {
    pub fn named(name: String, value: impl Into<PsValue>) -> Self {
        Self {
            name: Some(name),
            value: value.into(),
        }
    }

    pub fn positional(value: impl Into<PsValue>) -> Self {
        Self {
            name: None,
            value: value.into(),
        }
    }
}
