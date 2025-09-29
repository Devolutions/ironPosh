use serde::{Deserialize, Serialize};

use super::PsValue;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PsProperty {
    pub name: String,
    pub value: PsValue,
}
