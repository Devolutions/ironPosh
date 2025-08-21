use crate::ps_value::{ComplexObject, ComplexObjectContent, PsPrimitiveValue, PsProperty, PsValue};
use std::collections::BTreeMap;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, PartialEq, Eq, Default, TypedBuilder)]
pub struct Coordinates {
    #[builder(default = 0)]
    pub x: i32,
    #[builder(default = 0)]
    pub y: i32,
}

impl From<Coordinates> for ComplexObject {
    fn from(coords: Coordinates) -> Self {
        let mut extended_properties = BTreeMap::new();
        extended_properties.insert(
            "x".to_string(),
            PsProperty {
                name: "x".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(coords.x)),
            },
        );
        extended_properties.insert(
            "y".to_string(),
            PsProperty {
                name: "y".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(coords.y)),
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

#[derive(Debug, Clone, PartialEq, Eq, Default, TypedBuilder)]
pub struct Size {
    #[builder(default = 0)]
    pub width: i32,
    #[builder(default = 0)]
    pub height: i32,
}

impl From<Size> for ComplexObject {
    fn from(size: Size) -> Self {
        let mut extended_properties = BTreeMap::new();
        extended_properties.insert(
            "width".to_string(),
            PsProperty {
                name: "width".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(size.width)),
            },
        );
        extended_properties.insert(
            "height".to_string(),
            PsProperty {
                name: "height".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(size.height)),
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

#[derive(Debug, Clone, PartialEq, Eq, TypedBuilder)]
pub struct HostDefaultData {
    #[builder(default = 7)]
    pub foreground_color: i32, // Key 0: System.ConsoleColor
    #[builder(default = 0)]
    pub background_color: i32, // Key 1: System.ConsoleColor
    #[builder(default)]
    pub cursor_position: Coordinates, // Key 2: System.Management.Automation.Host.Coordinates
    #[builder(default)]
    pub window_position: Coordinates, // Key 3: System.Management.Automation.Host.Coordinates
    #[builder(default = 25)]
    pub max_physical_cursor_size: i32, // Key 4: System.Int32
    #[builder(default_code = "Size { width: 120, height: 30 }")]
    pub window_size: Size, // Key 5: System.Management.Automation.Host.Size
    #[builder(default_code = "Size { width: 120, height: 30 }")]
    pub buffer_size: Size, // Key 6: System.Management.Automation.Host.Size
    #[builder(default_code = "Size { width: 120, height: 30 }")]
    pub max_window_size: Size, // Key 7: System.Management.Automation.Host.Size
    #[builder(default_code = "Size { width: 3824, height: 2121 }")]
    pub max_physical_window_size: Size, // Key 8: System.Management.Automation.Host.Size
    #[builder(default = "PowerShell".to_string())]
    pub host_name: String, // Key 9: System.String
}

impl Default for HostDefaultData {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl HostDefaultData {

    // Convert to the BTreeMap<PsValue, PsValue> format expected by HostInfo DCT
    pub fn to_dictionary(&self) -> BTreeMap<PsValue, PsValue> {
        let mut map = BTreeMap::new();

        // Key 0: Foreground color
        let mut fg_props = BTreeMap::new();
        fg_props.insert(
            "T".to_string(),
            PsProperty {
                name: "T".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str("System.ConsoleColor".to_string())),
            },
        );
        fg_props.insert(
            "V".to_string(),
            PsProperty {
                name: "V".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(self.foreground_color)),
            },
        );
        map.insert(
            PsValue::Primitive(PsPrimitiveValue::I32(0)),
            PsValue::Object(ComplexObject {
                type_def: None,
                to_string: None,
                content: ComplexObjectContent::Standard,
                adapted_properties: BTreeMap::new(),
                extended_properties: fg_props,
            }),
        );

        // Simplified implementation - just add essential host name entry
        // Key 9: Host name
        let mut host_props = BTreeMap::new();
        host_props.insert(
            "T".to_string(),
            PsProperty {
                name: "T".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str("System.String".to_string())),
            },
        );
        host_props.insert(
            "V".to_string(),
            PsProperty {
                name: "V".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(self.host_name.clone())),
            },
        );
        map.insert(
            PsValue::Primitive(PsPrimitiveValue::I32(9)),
            PsValue::Object(ComplexObject {
                type_def: None,
                to_string: None,
                content: ComplexObjectContent::Standard,
                adapted_properties: BTreeMap::new(),
                extended_properties: host_props,
            }),
        );

        map
    }
}

// TODO: Add tests for new ComplexObject representation
