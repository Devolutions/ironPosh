use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize};
use std::collections::BTreeMap;

/// PSVersionTable entry of ApplicationArguments — a PSPrimitiveDictionary whose
/// values are macro-derived (Version values via `version_conv`, the compatible-
/// versions array via `version_array_conv`).
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(
    dictionary,
    type_names(
        "System.Management.Automation.PSPrimitiveDictionary",
        "System.Collections.Hashtable",
        "System.Object"
    )
)]
pub struct PSVersionTable {
    #[ps(name = "PSSemanticVersion", default)]
    pub ps_semantic_version: String,
    #[ps(name = "PSRemotingProtocolVersion", with = "version_conv", default)]
    pub ps_remoting_protocol_version: String,
    #[ps(name = "PSCompatibleVersions", with = "version_array_conv", default)]
    pub ps_compatible_versions: Vec<String>,
    #[ps(name = "WSManStackVersion", with = "version_conv", default)]
    pub wsman_stack_version: String,
    #[ps(name = "SerializationVersion", with = "version_conv", default)]
    pub serialization_version: String,
    #[ps(name = "OS", default)]
    pub os: String,
    #[ps(name = "PSEdition", default)]
    pub ps_edition: String,
    #[ps(name = "PSVersion", with = "version_conv", default)]
    pub ps_version: String,
    #[ps(name = "Platform", default)]
    pub platform: String,
    #[ps(name = "GitCommitId", default)]
    pub git_commit_id: String,
}

impl Default for PSVersionTable {
    fn default() -> Self {
        Self {
            ps_semantic_version: "7.4.11".to_string(),
            ps_remoting_protocol_version: "2.3".to_string(),
            ps_compatible_versions: vec![
                "1.0".to_string(),
                "2.0".to_string(),
                "3.0".to_string(),
                "4.0".to_string(),
                "5.0".to_string(),
                "5.1".to_string(),
                "6.0".to_string(),
                "7.0".to_string(),
            ],
            wsman_stack_version: "3.0".to_string(),
            serialization_version: "1.1.0.1".to_string(),
            os: "Microsoft Windows 10.0.22631".to_string(),
            ps_edition: "Core".to_string(),
            ps_version: "7.4.11".to_string(),
            platform: "Win32NT".to_string(),
            git_commit_id: "7.4.11".to_string(),
        }
    }
}

/// `#[ps(with)]`: a `Version` primitive value.
mod version_conv {
    use crate::PowerShellRemotingError;
    use crate::ps_value::{PsPrimitiveValue, PsValue};

    pub fn to_ps_value(value: &str) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::Version(value.to_string()))
    }

    #[allow(clippy::unnecessary_wraps)] // signature fixed by #[ps(with)]
    pub fn from_ps_value(value: &PsValue) -> Result<String, PowerShellRemotingError> {
        Ok(match value {
            PsValue::Primitive(PsPrimitiveValue::Version(v) | PsPrimitiveValue::Str(v)) => {
                v.clone()
            }
            _ => String::new(),
        })
    }
}

/// `#[ps(with)]`: a `System.Version[]` array of `Version` values.
mod version_array_conv {
    use crate::PowerShellRemotingError;
    use crate::ps_value::{
        ComplexObject, ComplexObjectContent, Container, Properties, PsPrimitiveValue, PsType,
        PsValue,
    };
    use std::borrow::Cow;

    pub fn to_ps_value(values: &[String]) -> PsValue {
        let items = values
            .iter()
            .map(|v| PsValue::Primitive(PsPrimitiveValue::Version(v.clone())))
            .collect();
        PsValue::Object(ComplexObject {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Version[]"),
                    Cow::Borrowed("System.Array"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: None,
            content: ComplexObjectContent::Container(Container::List(items)),
            properties: Properties::new(),
        })
    }

    #[allow(clippy::unnecessary_wraps)] // signature fixed by #[ps(with)]
    pub fn from_ps_value(value: &PsValue) -> Result<Vec<String>, PowerShellRemotingError> {
        let mut out = Vec::new();
        if let PsValue::Object(obj) = value
            && let ComplexObjectContent::Container(Container::List(items)) = &obj.content
        {
            for item in items {
                if let PsValue::Primitive(PsPrimitiveValue::Version(v) | PsPrimitiveValue::Str(v)) =
                    item
                {
                    out.push(v.clone());
                }
            }
        }
        Ok(out)
    }
}

/// ApplicationArguments (MS-PSRP §2.2.3.13): a PSPrimitiveDictionary carrying the
/// PSVersionTable plus any additional session arguments (flattened).
#[derive(
    Debug, Clone, Default, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize,
)]
#[ps(
    dictionary,
    type_names(
        "System.Management.Automation.PSPrimitiveDictionary",
        "System.Collections.Hashtable",
        "System.Object"
    )
)]
pub struct ApplicationArguments {
    #[ps(name = "PSVersionTable")]
    pub ps_version_table: Option<PSVersionTable>,
    #[builder(default)]
    #[ps(flatten)]
    pub additional_arguments: BTreeMap<String, PsValue>,
}

impl ApplicationArguments {
    /// Create an empty ApplicationArguments (renders as an empty dictionary).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Check if this ApplicationArguments is empty.
    pub fn is_empty(&self) -> bool {
        self.ps_version_table.is_none() && self.additional_arguments.is_empty()
    }
}
