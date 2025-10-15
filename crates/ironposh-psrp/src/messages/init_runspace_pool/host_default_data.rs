use crate::PowerShellRemotingError;
use crate::ps_value::{ComplexObject, ComplexObjectContent, PsPrimitiveValue, PsProperty, PsValue};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use typed_builder::TypedBuilder;

#[cfg(feature = "crossterm")]
use crossterm::{cursor, style::Color, terminal};

/// Represents a typed value wrapper that matches the PowerShell remoting protocol structure
/// where each value has a type (T) and value (V) property
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueWrapper {
    pub type_name: String,
    pub value: PsValue,
}

impl ValueWrapper {
    pub fn new_i32(value: i32, type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::I32(value)),
        }
    }

    pub fn new_string(value: &str) -> Self {
        Self {
            type_name: "System.String".to_string(),
            value: PsValue::Primitive(PsPrimitiveValue::Str(value.to_string())),
        }
    }

    pub fn new_coordinates(coords: &Coordinates) -> Self {
        Self {
            type_name: "System.Management.Automation.Host.Coordinates".to_string(),
            value: PsValue::Object(coords.clone().into()),
        }
    }

    pub fn new_size(size: &Size) -> Self {
        Self {
            type_name: "System.Management.Automation.Host.Size".to_string(),
            value: PsValue::Object(size.clone().into()),
        }
    }
}

impl From<ValueWrapper> for ComplexObject {
    fn from(wrapper: ValueWrapper) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "T".to_string(),
            PsProperty {
                name: "T".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(wrapper.type_name)),
            },
        );

        extended_properties.insert(
            "V".to_string(),
            PsProperty {
                name: "V".to_string(),
                value: wrapper.value,
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

impl TryFrom<&ComplexObject> for ValueWrapper {
    type Error = PowerShellRemotingError;

    fn try_from(obj: &ComplexObject) -> Result<Self, Self::Error> {
        let type_name = obj
            .extended_properties
            .get("T")
            .and_then(|p| match &p.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                PowerShellRemotingError::InvalidMessage(
                    "Missing or invalid type property 'T' in ValueWrapper".to_string(),
                )
            })?;

        let value = obj
            .extended_properties
            .get("V")
            .map(|p| p.value.clone())
            .ok_or_else(|| {
                PowerShellRemotingError::InvalidMessage(
                    "Missing value property 'V' in ValueWrapper".to_string(),
                )
            })?;

        Ok(Self { type_name, value })
    }
}

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
        Self {
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

        Ok(Self {
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
        Self {
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

        Ok(Self {
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
    pub buffer_size: Size, // Key 5: System.Management.Automation.Host.Size (screen buffer)
    pub window_size: Size, // Key 6: System.Management.Automation.Host.Size (view window)
    pub max_window_size: Size, // Key 7: System.Management.Automation.Host.Size
    pub max_physical_window_size: Size, // Key 8: System.Management.Automation.Host.Size
    #[builder(default = "PowerShell".to_string())]
    pub window_title: String, // Key 9: System.String
    #[builder(default = "en-US".to_string())]
    pub locale: String, // Key 10: System.String
    #[builder(default = "en-US".to_string())]
    pub ui_locale: String, // Key 11: System.String
}

#[cfg(feature = "crossterm")]
fn console_color_to_i32(color: Color) -> i32 {
    match color {
        Color::Black => 0,
        Color::DarkBlue => 1,
        Color::DarkGreen => 2,
        Color::DarkCyan => 3,
        Color::DarkRed => 4,
        Color::DarkMagenta => 5,
        Color::DarkYellow => 6,
        // Map non-16-color values to nearest (Grey as fallback)
        Color::Grey | Color::Rgb { .. } | Color::AnsiValue(_) | Color::Reset => 7,
        Color::DarkGrey => 8,
        Color::Blue => 9,
        Color::Green => 10,
        Color::Cyan => 11,
        Color::Red => 12,
        Color::Magenta => 13,
        Color::Yellow => 14,
        Color::White => 15,
    }
}

impl HostDefaultData {
    /// Creates HostDefaultData from current crossterm terminal state
    ///
    /// Queries the terminal for current cursor position and buffer size,
    /// uses sensible defaults for values crossterm cannot provide.
    ///
    /// # Errors
    ///
    /// Returns error if crossterm fails to query terminal state.
    #[cfg(feature = "crossterm")]
    pub fn from_crossterm() -> Result<Self, std::io::Error> {
        // Query terminal state
        let (cols, rows) = terminal::size()?;
        let (cursor_x, cursor_y) = cursor::position()?;

        // Choose default colors (can be customized by caller)
        let fg_color = Color::Grey; // -> 7
        let bg_color = Color::Black; // -> 0

        // Convert to console color integers
        let foreground_color = console_color_to_i32(fg_color);
        let background_color = console_color_to_i32(bg_color);

        // Convert terminal dimensions to i32
        let cols = cols as i32;
        let rows = rows as i32;
        let x = cursor_x as i32;
        let y = cursor_y as i32;

        Ok(Self {
            foreground_color,
            background_color,
            cursor_position: Coordinates { x, y },
            window_position: Coordinates { x: 0, y: 0 }, // Not exposed by crossterm
            cursor_size: 25,                             // Default cursor size (0-100%)
            buffer_size: Size {
                width: cols,
                height: rows,
            },
            window_size: Size {
                width: cols,
                height: rows,
            },
            max_window_size: Size {
                width: cols,
                height: rows,
            },
            max_physical_window_size: Size {
                width: cols,
                height: rows,
            },
            window_title: "PowerShell".to_string(),
            locale: "en-US".to_string(),
            ui_locale: "en-US".to_string(),
        })
    }

    // Convert to the BTreeMap<PsValue, PsValue> format expected by HostInfo DCT
    pub fn to_dictionary(&self) -> BTreeMap<PsValue, PsValue> {
        let mut map = BTreeMap::new();

        // Helper function to add wrapped values to the map
        let add_wrapped_value = |map: &mut BTreeMap<_, _>, key: i32, wrapper: ValueWrapper| {
            map.insert(
                PsValue::Primitive(PsPrimitiveValue::I32(key)),
                PsValue::Object(wrapper.into()),
            );
        };

        // Add all values wrapped in ValueWrapper objects
        add_wrapped_value(
            &mut map,
            0,
            ValueWrapper::new_i32(self.foreground_color, "System.ConsoleColor"),
        );
        add_wrapped_value(
            &mut map,
            1,
            ValueWrapper::new_i32(self.background_color, "System.ConsoleColor"),
        );
        add_wrapped_value(
            &mut map,
            2,
            ValueWrapper::new_coordinates(&self.cursor_position),
        );
        add_wrapped_value(
            &mut map,
            3,
            ValueWrapper::new_coordinates(&self.window_position),
        );
        add_wrapped_value(
            &mut map,
            4,
            ValueWrapper::new_i32(self.cursor_size, "System.Int32"),
        );
        add_wrapped_value(&mut map, 5, ValueWrapper::new_size(&self.buffer_size));
        add_wrapped_value(&mut map, 6, ValueWrapper::new_size(&self.window_size));
        add_wrapped_value(&mut map, 7, ValueWrapper::new_size(&self.max_window_size));
        add_wrapped_value(
            &mut map,
            8,
            ValueWrapper::new_size(&self.max_physical_window_size),
        );
        add_wrapped_value(&mut map, 9, ValueWrapper::new_string(&self.window_title));
        add_wrapped_value(&mut map, 10, ValueWrapper::new_string(&self.locale));
        add_wrapped_value(&mut map, 11, ValueWrapper::new_string(&self.ui_locale));

        map
    }
}

impl TryFrom<BTreeMap<PsValue, PsValue>> for HostDefaultData {
    type Error = PowerShellRemotingError;

    fn try_from(dict: BTreeMap<PsValue, PsValue>) -> Result<Self, Self::Error> {
        // Helper function to extract ValueWrapper from the dictionary
        let get_value_wrapper = |key: i32| -> Result<ValueWrapper, Self::Error> {
            dict.get(&PsValue::Primitive(PsPrimitiveValue::I32(key)))
                .and_then(|v| match v {
                    PsValue::Object(obj) => ValueWrapper::try_from(obj).ok(),
                    PsValue::Primitive(_) => None,
                })
                .ok_or_else(|| {
                    Self::Error::InvalidMessage(format!(
                        "Missing or invalid ValueWrapper for key {key}"
                    ))
                })
        };

        // Helper functions to extract typed values from ValueWrapper
        let get_i32_from_wrapper = |key: i32| -> Result<i32, Self::Error> {
            let wrapper = get_value_wrapper(key)?;
            match wrapper.value {
                PsValue::Primitive(PsPrimitiveValue::I32(val)) => Ok(val),
                _ => Err(Self::Error::InvalidMessage(format!(
                    "Expected i32 value for key {key}"
                ))),
            }
        };

        let get_string_from_wrapper = |key: i32| -> Result<String, Self::Error> {
            let wrapper = get_value_wrapper(key)?;
            match wrapper.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Ok(s),
                _ => Err(Self::Error::InvalidMessage(format!(
                    "Expected string value for key {key}"
                ))),
            }
        };

        let get_coords_from_wrapper = |key: i32| -> Result<Coordinates, Self::Error> {
            let wrapper = get_value_wrapper(key)?;
            match wrapper.value {
                PsValue::Object(obj) => Coordinates::try_from(&obj),
                PsValue::Primitive(_) => Err(Self::Error::InvalidMessage(format!(
                    "Expected Coordinates object for key {key}"
                ))),
            }
        };

        let get_size_from_wrapper = |key: i32| -> Result<Size, Self::Error> {
            let wrapper = get_value_wrapper(key)?;
            match wrapper.value {
                PsValue::Object(obj) => Size::try_from(&obj),
                PsValue::Primitive(_) => Err(Self::Error::InvalidMessage(format!(
                    "Expected Size object for key {key}"
                ))),
            }
        };

        Ok(Self {
            foreground_color: get_i32_from_wrapper(0)?,
            background_color: get_i32_from_wrapper(1)?,
            cursor_position: get_coords_from_wrapper(2)?,
            window_position: get_coords_from_wrapper(3)?,
            cursor_size: get_i32_from_wrapper(4)?,
            buffer_size: get_size_from_wrapper(5)?,
            window_size: get_size_from_wrapper(6)?,
            max_window_size: get_size_from_wrapper(7)?,
            max_physical_window_size: get_size_from_wrapper(8)?,
            window_title: get_string_from_wrapper(9)?,
            locale: get_string_from_wrapper(10)?,
            ui_locale: get_string_from_wrapper(11)?,
        })
    }
}
