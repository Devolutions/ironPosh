use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize};
use std::collections::BTreeMap;

/// APPLICATION_PRIVATE_DATA (MS-PSRP §2.2.2.13): server → client.
///
/// Carries an extended property `ApplicationPrivateData` holding either a
/// `PSPrimitiveDictionary` (arbitrary application data) or `Nil`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(message_type = ApplicationPrivateData)]
pub struct ApplicationPrivateData {
    /// The application private data as a dictionary of string keys to values.
    #[ps(name = "ApplicationPrivateData", nil_when_none)]
    pub data: Option<BTreeMap<String, PsValue>>,
}

impl ApplicationPrivateData {
    /// Create a new `ApplicationPrivateData` with no data (null value).
    pub fn new() -> Self {
        Self::default()
    }
}
