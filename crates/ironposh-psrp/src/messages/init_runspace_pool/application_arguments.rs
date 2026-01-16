use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsType, PsValue,
};
use std::{borrow::Cow, collections::BTreeMap};

/// Represents the PSVersionTable entry within ApplicationArguments
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct PSVersionTable {
    pub ps_semantic_version: String,
    pub ps_remoting_protocol_version: String,
    pub ps_compatible_versions: Vec<String>,
    pub wsman_stack_version: String,
    pub serialization_version: String,
    pub os: String,
    pub ps_edition: String,
    pub ps_version: String,
    pub platform: String,
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

impl From<PSVersionTable> for ComplexObject {
    fn from(version_table: PSVersionTable) -> Self {
        let mut entries = BTreeMap::new();

        // PSSemanticVersion (as String)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("PSSemanticVersion".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Str(version_table.ps_semantic_version)),
        );

        // PSRemotingProtocolVersion (as Version)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str(
                "PSRemotingProtocolVersion".to_string(),
            )),
            PsValue::Primitive(PsPrimitiveValue::Version(
                version_table.ps_remoting_protocol_version,
            )),
        );

        // PSCompatibleVersions as an array
        let compatible_versions: Vec<PsValue> = version_table
            .ps_compatible_versions
            .into_iter()
            .map(|v| PsValue::Primitive(PsPrimitiveValue::Version(v)))
            .collect();

        let compatible_versions_obj = Self {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Version[]"),
                    Cow::Borrowed("System.Array"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: None,
            content: ComplexObjectContent::Container(Container::List(compatible_versions)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        };

        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("PSCompatibleVersions".to_string())),
            PsValue::Object(compatible_versions_obj),
        );

        // WSManStackVersion (as Version)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("WSManStackVersion".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Version(version_table.wsman_stack_version)),
        );

        // SerializationVersion (as Version)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("SerializationVersion".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Version(
                version_table.serialization_version,
            )),
        );

        // OS (as String)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("OS".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Str(version_table.os)),
        );

        // PSEdition (as String)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("PSEdition".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Str(version_table.ps_edition)),
        );

        // PSVersion (as Version)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("PSVersion".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Version(version_table.ps_version)),
        );

        // Platform (as String)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("Platform".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Str(version_table.platform)),
        );

        // GitCommitId (as String)
        entries.insert(
            PsValue::Primitive(PsPrimitiveValue::Str("GitCommitId".to_string())),
            PsValue::Primitive(PsPrimitiveValue::Str(version_table.git_commit_id)),
        );

        Self {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Management.Automation.PSPrimitiveDictionary"),
                    Cow::Borrowed("System.Collections.Hashtable"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: None,
            content: ComplexObjectContent::Container(Container::Dictionary(entries)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        }
    }
}

impl TryFrom<ComplexObject> for PSVersionTable {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let ComplexObjectContent::Container(Container::Dictionary(entries)) = value.content else {
            return Err(Self::Error::InvalidMessage(
                "Expected Dictionary for PSVersionTable".to_string(),
            ));
        };

        let get_string_value = |key: &str| -> Result<String, Self::Error> {
            let key_value = PsValue::Primitive(PsPrimitiveValue::Str(key.to_string()));
            match entries.get(&key_value) {
                Some(PsValue::Primitive(
                    PsPrimitiveValue::Str(s) | PsPrimitiveValue::Version(s),
                )) => Ok(s.clone()),
                Some(_) => Err(Self::Error::InvalidMessage(format!(
                    "Property '{key}' is not a String or Version"
                ))),
                None => Err(Self::Error::InvalidMessage(format!(
                    "Missing property: {key}"
                ))),
            }
        };

        let ps_semantic_version = get_string_value("PSSemanticVersion")?;
        let ps_remoting_protocol_version = get_string_value("PSRemotingProtocolVersion")?;
        let wsman_stack_version = get_string_value("WSManStackVersion")?;
        let serialization_version = get_string_value("SerializationVersion")?;
        let os = get_string_value("OS")?;
        let ps_edition = get_string_value("PSEdition")?;
        let ps_version = get_string_value("PSVersion")?;
        let platform = get_string_value("Platform")?;
        let git_commit_id = get_string_value("GitCommitId")?;

        // Handle PSCompatibleVersions array
        let ps_compatible_versions = {
            let key_value =
                PsValue::Primitive(PsPrimitiveValue::Str("PSCompatibleVersions".to_string()));
            match entries.get(&key_value) {
                Some(PsValue::Object(obj)) => match &obj.content {
                    ComplexObjectContent::Container(Container::List(versions)) => {
                        let mut version_strings = Vec::new();
                        for version in versions {
                            match version {
                                PsValue::Primitive(
                                    PsPrimitiveValue::Str(s) | PsPrimitiveValue::Version(s),
                                ) => {
                                    version_strings.push(s.clone());
                                }
                                _ => {
                                    return Err(Self::Error::InvalidMessage(
                                        "PSCompatibleVersions contains non-string/version value"
                                            .to_string(),
                                    ));
                                }
                            }
                        }
                        version_strings
                    }
                    _ => {
                        return Err(Self::Error::InvalidMessage(
                            "PSCompatibleVersions is not a List".to_string(),
                        ));
                    }
                },
                Some(_) => {
                    return Err(Self::Error::InvalidMessage(
                        "PSCompatibleVersions is not an Object".to_string(),
                    ));
                }
                None => {
                    return Err(Self::Error::InvalidMessage(
                        "Missing property: PSCompatibleVersions".to_string(),
                    ));
                }
            }
        };

        Ok(Self {
            ps_semantic_version,
            ps_remoting_protocol_version,
            ps_compatible_versions,
            wsman_stack_version,
            serialization_version,
            os,
            ps_edition,
            ps_version,
            platform,
            git_commit_id,
        })
    }
}

/// Represents the ApplicationArguments structure in PowerShell Remoting
#[derive(Debug, Clone, Default, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct ApplicationArguments {
    pub ps_version_table: Option<PSVersionTable>,
    pub additional_arguments: BTreeMap<String, PsValue>,
}

impl ApplicationArguments {
    /// Create an empty ApplicationArguments (renders as Nil in XML)
    pub fn empty() -> Self {
        Self {
            ps_version_table: None,
            additional_arguments: BTreeMap::new(),
        }
    }

    /// Check if this ApplicationArguments is empty
    pub fn is_empty(&self) -> bool {
        self.ps_version_table.is_none() && self.additional_arguments.is_empty()
    }
}

impl From<ApplicationArguments> for ComplexObject {
    fn from(app_args: ApplicationArguments) -> Self {
        let mut entries = BTreeMap::new();

        // Add PSVersionTable if present
        if let Some(version_table) = app_args.ps_version_table {
            entries.insert(
                PsValue::Primitive(PsPrimitiveValue::Str("PSVersionTable".to_string())),
                PsValue::Object(version_table.into()),
            );
        }

        // Add any additional arguments
        for (key, value) in app_args.additional_arguments {
            entries.insert(PsValue::Primitive(PsPrimitiveValue::Str(key)), value);
        }

        Self {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Management.Automation.PSPrimitiveDictionary"),
                    Cow::Borrowed("System.Collections.Hashtable"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: None,
            content: ComplexObjectContent::Container(Container::Dictionary(entries)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        }
    }
}

impl TryFrom<ComplexObject> for ApplicationArguments {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let ComplexObjectContent::Container(Container::Dictionary(entries)) = value.content else {
            return Err(Self::Error::InvalidMessage(
                "Expected Dictionary for ApplicationArguments".to_string(),
            ));
        };

        let mut ps_version_table = None;
        let mut additional_arguments = BTreeMap::new();

        for (key, value) in entries {
            match &key {
                PsValue::Primitive(PsPrimitiveValue::Str(key_str)) => match key_str.as_str() {
                    "PSVersionTable" => match value {
                        PsValue::Object(obj) => {
                            ps_version_table = Some(PSVersionTable::try_from(obj)?);
                        }
                        PsValue::Primitive(_) => {
                            return Err(Self::Error::InvalidMessage(
                                "PSVersionTable is not an Object".to_string(),
                            ));
                        }
                    },
                    _ => {
                        additional_arguments.insert(key_str.clone(), value);
                    }
                },
                _ => {
                    return Err(Self::Error::InvalidMessage(
                        "ApplicationArguments key is not a string".to_string(),
                    ));
                }
            }
        }

        Ok(Self {
            ps_version_table,
            additional_arguments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ps_version_table_serialization_deserialization() {
        let original_version_table = PSVersionTable::default();

        let complex_object: ComplexObject = original_version_table.clone().into();
        let deserialized_version_table = PSVersionTable::try_from(complex_object).unwrap();

        assert_eq!(original_version_table, deserialized_version_table);
    }

    #[test]
    fn test_application_arguments_empty() {
        let empty_args = ApplicationArguments::empty();
        assert!(empty_args.is_empty());
    }

    #[test]
    fn test_application_arguments_with_version_table() {
        // Create ApplicationArguments with a version table
        let args = ApplicationArguments {
            ps_version_table: Some(PSVersionTable::default()),
            additional_arguments: BTreeMap::new(),
        };
        assert!(!args.is_empty());
        assert!(args.ps_version_table.is_some());
    }

    #[test]
    fn test_application_arguments_serialization_deserialization() {
        let original_args = ApplicationArguments::default();

        let complex_object: ComplexObject = original_args.clone().into();
        let deserialized_args = ApplicationArguments::try_from(complex_object).unwrap();

        assert_eq!(original_args, deserialized_args);
    }

    #[test]
    fn test_application_arguments_with_additional_args() {
        let mut args = ApplicationArguments::default();
        args.additional_arguments.insert(
            "CustomKey".to_string(),
            PsValue::Primitive(PsPrimitiveValue::Str("CustomValue".to_string())),
        );

        let complex_object: ComplexObject = args.clone().into();
        let deserialized_args = ApplicationArguments::try_from(complex_object).unwrap();

        assert_eq!(args, deserialized_args);
    }
}
