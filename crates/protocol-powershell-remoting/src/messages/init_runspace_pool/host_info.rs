use super::HostDefaultData;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsProperty, PsType, PsValue,
};
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq, Default, typed_builder::TypedBuilder)]
pub struct HostInfo {
    #[builder(default = false)]
    pub is_host_null: bool,
    #[builder(default = false)]
    pub is_host_ui_null: bool,
    #[builder(default = false)]
    pub is_host_raw_ui_null: bool,
    #[builder(default = false)]
    pub use_runspace_host: bool,
    #[builder(default)]
    pub host_default_data: HostDefaultData,
}

impl From<HostInfo> for ComplexObject {
    fn from(host_info: HostInfo) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "_isHostNull".to_string(),
            PsProperty {
                name: "_isHostNull".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(host_info.is_host_null)),
            },
        );

        extended_properties.insert(
            "_isHostUINull".to_string(),
            PsProperty {
                name: "_isHostUINull".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(host_info.is_host_ui_null)),
            },
        );

        extended_properties.insert(
            "_isHostRawUINull".to_string(),
            PsProperty {
                name: "_isHostRawUINull".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(host_info.is_host_raw_ui_null)),
            },
        );

        extended_properties.insert(
            "_useRunspaceHost".to_string(),
            PsProperty {
                name: "_useRunspaceHost".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(host_info.use_runspace_host)),
            },
        );

        let host_default_data = host_info.host_default_data;
        let data_props = BTreeMap::from([(
            "data".to_string(),
            PsProperty {
                name: "data".to_string(),
                value: PsValue::Object(ComplexObject {
                    type_def: Some(PsType {
                        type_names: vec![
                            Cow::Borrowed("System.Collections.Hashtable"),
                            Cow::Borrowed("System.Object"),
                        ],
                    }),
                    to_string: None,
                    content: ComplexObjectContent::Container(Container::Dictionary(
                        host_default_data.to_dictionary(),
                    )),
                    adapted_properties: BTreeMap::new(),
                    extended_properties: BTreeMap::new(),
                }),
            },
        )]);

        let host_data_obj = ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties: data_props,
        };

        extended_properties.insert(
            "_hostDefaultData".to_string(),
            PsProperty {
                name: "_hostDefaultData".to_string(),
                value: PsValue::Object(host_data_obj),
            },
        );

        ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for HostInfo {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_bool_property = |name: &str| -> Result<bool, Self::Error> {
            let property = value
                .extended_properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))?;

            match &property.value {
                PsValue::Primitive(PsPrimitiveValue::Bool(b)) => Ok(*b),
                _ => Err(Self::Error::InvalidMessage(format!(
                    "Property '{name}' is not a Bool"
                ))),
            }
        };

        let is_host_null = get_bool_property("_isHostNull").unwrap_or(false);
        let is_host_ui_null = get_bool_property("_isHostUINull").unwrap_or(false);
        let is_host_raw_ui_null = get_bool_property("_isHostRawUINull").unwrap_or(false);
        let use_runspace_host = get_bool_property("_useRunspaceHost").unwrap_or(false);

        // For now, use default HostDefaultData since it's complex to deserialize
        // and the real XML example doesn't seem to include the full host default data
        let host_default_data = HostDefaultData::default();

        Ok(HostInfo {
            is_host_null,
            is_host_ui_null,
            is_host_raw_ui_null,
            use_runspace_host,
            host_default_data,
        })
    }
}

// TODO: Add tests for new ComplexObject representation
