use super::{Coordinates, HostDefaultData, Size};
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsProperty, PsType, PsValue,
};
use std::{borrow::Cow, collections::BTreeMap};

#[expect(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct HostInfo {
    #[builder(default = false)]
    pub is_host_null: bool,
    #[builder(default = false)]
    pub is_host_ui_null: bool,
    #[builder(default = false)]
    pub is_host_raw_ui_null: bool,
    #[builder(default = false)]
    pub use_runspace_host: bool,
    pub host_default_data: HostDefaultData,
}

impl HostInfo {
    pub fn enabled_all(host_data: HostDefaultData) -> Self {
        Self {
            is_host_null: true,
            is_host_ui_null: true,
            is_host_raw_ui_null: true,
            use_runspace_host: true,
            host_default_data: host_data,
        }
    }
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
                value: PsValue::Object(Self {
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

        let host_data_obj = Self {
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

        Self {
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

        let host_default_data = match value.extended_properties.get("_hostDefaultData") {
            Some(prop) => match &prop.value {
                PsValue::Object(host_data_obj) => {
                    let data_prop =
                        host_data_obj
                            .extended_properties
                            .get("data")
                            .ok_or_else(|| {
                                Self::Error::InvalidMessage(
                                    "Missing property: data in _hostDefaultData".to_string(),
                                )
                            })?;
                    match &data_prop.value {
                        PsValue::Object(data_obj) => match &data_obj.content {
                            ComplexObjectContent::Container(Container::Dictionary(dict)) => {
                                HostDefaultData::try_from(dict.clone())
                            }
                            _ => Err(Self::Error::InvalidMessage(
                                "Expected Dictionary for data property content".to_string(),
                            )),
                        },
                        PsValue::Primitive(_) => Err(Self::Error::InvalidMessage(
                            "Expected Object for data property".to_string(),
                        )),
                    }
                }
                PsValue::Primitive(_) => Err(Self::Error::InvalidMessage(
                    "Expected Object for _hostDefaultData property".to_string(),
                )),
            }?,
            None => HostDefaultData {
                foreground_color: 7,
                background_color: 0,
                cursor_position: Coordinates::default(),
                window_position: Coordinates::default(),
                cursor_size: 25,
                buffer_size: Size {
                    width: 120,
                    height: 3000,
                },
                window_size: Size {
                    width: 120,
                    height: 50,
                },
                max_window_size: Size {
                    width: 120,
                    height: 50,
                },
                max_physical_window_size: Size {
                    width: 120,
                    height: 50,
                },
                window_title: "PowerShell".to_string(),
                locale: "en-US".to_string(),
                ui_locale: "en-US".to_string(),
            },
        };

        Ok(Self {
            is_host_null,
            is_host_ui_null,
            is_host_raw_ui_null,
            use_runspace_host,
            host_default_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::init_runspace_pool::{Coordinates, Size};

    #[test]
    fn test_host_info_serialization_deserialization() {
        let original_host_info = HostInfo {
            is_host_null: false,
            is_host_ui_null: false,
            is_host_raw_ui_null: false,
            use_runspace_host: true,
            host_default_data: HostDefaultData {
                foreground_color: 7,
                background_color: 0,
                cursor_position: Coordinates { x: 0, y: 0 },
                window_position: Coordinates { x: 0, y: 0 },
                cursor_size: 25,
                window_size: Size {
                    width: 120,
                    height: 50,
                },
                buffer_size: Size {
                    width: 120,
                    height: 3000,
                },
                max_window_size: Size {
                    width: 120,
                    height: 50,
                },
                max_physical_window_size: Size {
                    width: 120,
                    height: 50,
                },
                window_title: "PowerShell".to_string(),
                locale: "en-US".to_string(),
                ui_locale: "en-US".to_string(),
            },
        };

        let complex_object: ComplexObject = original_host_info.clone().into();
        let deserialized_host_info = HostInfo::try_from(complex_object).unwrap();

        assert_eq!(original_host_info, deserialized_host_info);
    }
}

// TODO: Add tests for new ComplexObject representation
