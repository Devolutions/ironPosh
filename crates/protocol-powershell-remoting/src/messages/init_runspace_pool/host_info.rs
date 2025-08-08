use super::super::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsProperty, PsType, PsValue,
};
use super::HostDefaultData;
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostInfo {
    pub is_host_null: bool,
    pub is_host_ui_null: bool,
    pub is_host_raw_ui_null: bool,
    pub use_runspace_host: bool,
    pub host_default_data: Option<HostDefaultData>,
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

        if let Some(host_default_data) = host_info.host_default_data {
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
        }

        ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

// TODO: Add tests for new ComplexObject representation
