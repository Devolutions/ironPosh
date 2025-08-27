use super::PsValue;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PsProperty {
    pub name: String,
    pub value: PsValue,
}
