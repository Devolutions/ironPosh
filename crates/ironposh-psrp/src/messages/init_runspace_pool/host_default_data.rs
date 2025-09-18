use crate::PowerShellRemotingError;
use crate::ps_value::{ComplexObject, ComplexObjectContent, PsPrimitiveValue, PsProperty, PsValue};
use std::collections::BTreeMap;
use std::convert::TryFrom;
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

impl TryFrom<&ComplexObject> for Coordinates {
    type Error = PowerShellRemotingError;

    fn try_from(obj: &ComplexObject) -> Result<Self, Self::Error> {
        let get_i32 = |name: &str| {
            obj.extended_properties
                .get(name)
                .and_then(|p| match &p.value {
                    PsValue::Primitive(PsPrimitiveValue::I32(val)) => Some(*val),
                    _ => None,
                })
                .ok_or_else(|| {
                    PowerShellRemotingError::InvalidMessage(format!(
                        "Missing or invalid property '{name}' in Coordinates"
                    ))
                })
        };

        Ok(Coordinates {
            x: get_i32("x")?,
            y: get_i32("y")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Size {
    pub width: i32,
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

impl TryFrom<&ComplexObject> for Size {
    type Error = PowerShellRemotingError;

    fn try_from(obj: &ComplexObject) -> Result<Self, Self::Error> {
        let get_i32 = |name: &str| {
            obj.extended_properties
                .get(name)
                .and_then(|p| match &p.value {
                    PsValue::Primitive(PsPrimitiveValue::I32(val)) => Some(*val),
                    _ => None,
                })
                .ok_or_else(|| {
                    PowerShellRemotingError::InvalidMessage(format!(
                        "Missing or invalid property '{name}' in Size"
                    ))
                })
        };

        Ok(Size {
            width: get_i32("width")?,
            height: get_i32("height")?,
        })
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
    pub cursor_size: i32, // Key 4: System.Int32
    pub window_size: Size, // Key 5: System.Management.Automation.Host.Size
    pub buffer_size: Size, // Key 6: System.Management.Automation.Host.Size
    #[builder(default_code = "Size { width: 120, height: 30 }")]
    pub max_window_size: Size, // Key 7: System.Management.Automation.Host.Size
    #[builder(default_code = "Size { width: 3824, height: 2121 }")]
    pub max_physical_window_size: Size, // Key 8: System.Management.Automation.Host.Size
    #[builder(default = "PowerShell".to_string())]
    pub window_title: String, // Key 9: System.String
    #[builder(default = "en-US".to_string())]
    pub locale: String, // Key 10: System.String
    #[builder(default = "en-US".to_string())]
    pub ui_locale: String, // Key 11: System.String
}

impl HostDefaultData {
    // Convert to the BTreeMap<PsValue, PsValue> format expected by HostInfo DCT
    pub fn to_dictionary(&self) -> BTreeMap<PsValue, PsValue> {
        let mut map = BTreeMap::new();

        let add_i32 = |map: &mut BTreeMap<_, _>, key, val| {
            map.insert(
                PsValue::Primitive(PsPrimitiveValue::I32(key)),
                PsValue::Primitive(PsPrimitiveValue::I32(val)),
            );
        };

        let add_string = |map: &mut BTreeMap<_, _>, key, val: &str| {
            map.insert(
                PsValue::Primitive(PsPrimitiveValue::I32(key)),
                PsValue::Primitive(PsPrimitiveValue::Str(val.to_string())),
            );
        };

        let add_coords = |map: &mut BTreeMap<_, _>, key, val: &Coordinates| {
            map.insert(
                PsValue::Primitive(PsPrimitiveValue::I32(key)),
                PsValue::Object(val.clone().into()),
            );
        };

        let add_size = |map: &mut BTreeMap<_, _>, key, val: &Size| {
            map.insert(
                PsValue::Primitive(PsPrimitiveValue::I32(key)),
                PsValue::Object(val.clone().into()),
            );
        };

        add_i32(&mut map, 0, self.foreground_color);
        add_i32(&mut map, 1, self.background_color);
        add_coords(&mut map, 2, &self.cursor_position);
        add_coords(&mut map, 3, &self.window_position);
        add_i32(&mut map, 4, self.cursor_size);
        add_size(&mut map, 5, &self.window_size);
        add_size(&mut map, 6, &self.buffer_size);
        add_size(&mut map, 7, &self.max_window_size);
        add_size(&mut map, 8, &self.max_physical_window_size);
        add_string(&mut map, 9, &self.window_title);
        add_string(&mut map, 10, &self.locale);
        add_string(&mut map, 11, &self.ui_locale);

        map
    }
}

impl TryFrom<BTreeMap<PsValue, PsValue>> for HostDefaultData {
    type Error = PowerShellRemotingError;

    fn try_from(dict: BTreeMap<PsValue, PsValue>) -> Result<Self, Self::Error> {
        let get_i32 = |key: i32| -> Result<i32, Self::Error> {
            dict.get(&PsValue::Primitive(PsPrimitiveValue::I32(key)))
                .and_then(|v| match v {
                    PsValue::Primitive(PsPrimitiveValue::I32(val)) => Some(*val),
                    _ => None,
                })
                .ok_or_else(|| {
                    Self::Error::InvalidMessage(format!("Missing or invalid value for key {key}"))
                })
        };

        let get_string = |key: i32| -> Result<String, Self::Error> {
            dict.get(&PsValue::Primitive(PsPrimitiveValue::I32(key)))
                .and_then(|v| match v {
                    PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    Self::Error::InvalidMessage(format!("Missing or invalid value for key {key}"))
                })
        };

        let get_coords = |key: i32| -> Result<Coordinates, Self::Error> {
            dict.get(&PsValue::Primitive(PsPrimitiveValue::I32(key)))
                .and_then(|v| match v {
                    PsValue::Object(obj) => Coordinates::try_from(obj).ok(),
                    _ => None,
                })
                .ok_or_else(|| {
                    Self::Error::InvalidMessage(format!("Missing or invalid value for key {key}"))
                })
        };

        let get_size = |key: i32| -> Result<Size, Self::Error> {
            dict.get(&PsValue::Primitive(PsPrimitiveValue::I32(key)))
                .and_then(|v| match v {
                    PsValue::Object(obj) => Size::try_from(obj).ok(),
                    _ => None,
                })
                .ok_or_else(|| {
                    Self::Error::InvalidMessage(format!("Missing or invalid value for key {key}"))
                })
        };

        Ok(HostDefaultData {
            foreground_color: get_i32(0)?,
            background_color: get_i32(1)?,
            cursor_position: get_coords(2)?,
            window_position: get_coords(3)?,
            cursor_size: get_i32(4)?,
            window_size: get_size(5)?,
            buffer_size: get_size(6)?,
            max_window_size: get_size(7)?,
            max_physical_window_size: get_size(8)?,
            window_title: get_string(9)?,
            locale: get_string(10)?,
            ui_locale: get_string(11)?,
        })
    }
}

// TODO: Add tests for new ComplexObject representation
