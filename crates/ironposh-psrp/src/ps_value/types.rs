use core::hash;
use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/2784bd9c-267d-4297-b603-722c727f85f1
#[derive(Debug, Clone, Eq, Default, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PsType {
    /// The <TN> element contains <T> elements, each of which contains the name of a type associated with the object being serialized.
    /// <T> elements MUST be ordered from the most specific (that is, point) to least specific (that is, object).
    /// Type names MUST be encoded as described in section 2.2.5.3.2.
    ///  Mapping type names to concrete types is outside the scope of the protocol and is an implementation detail.
    pub type_names: Vec<Cow<'static, str>>,
}

impl PsType {
    pub fn ps_primitive_dictionary() -> Self {
        Self {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.PSPrimitiveDictionary"),
                Cow::Borrowed("System.Collections.Hashtable"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }

    pub fn remote_host_method_id() -> Self {
        Self {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.Remoting.RemoteHostMethodId"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }

    pub fn array_list() -> Self {
        Self {
            type_names: vec![
                Cow::Borrowed("System.Collections.ArrayList"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }

    pub fn pipeline_result_types() -> Self {
        Self {
            type_names: vec![
                Cow::Borrowed("System.Management.Automation.Runspaces.PipelineResultTypes"),
                Cow::Borrowed("System.Enum"),
                Cow::Borrowed("System.ValueType"),
                Cow::Borrowed("System.Object"),
            ],
        }
    }
}

impl PartialEq for PsType {
    fn eq(&self, other: &Self) -> bool {
        for (ty1, ty2) in self.type_names.iter().zip(other.type_names.iter()) {
            if ty1.as_ref() != ty2.as_ref() {
                return false;
            }
        }
        true
    }
}

impl hash::Hash for PsType {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        for ty in &self.type_names {
            ty.hash(state);
        }
    }
}
