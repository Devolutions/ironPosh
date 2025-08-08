use crate::{PsObject, PsProperty, PsValue};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Coordinates {
    pub x: i32,
    pub y: i32,
}

impl From<Coordinates> for PsObject {
    fn from(coords: Coordinates) -> Self {
        PsObject {
            ms: vec![
                PsProperty {
                    name: Some("x".to_string()),
                    ref_id: None,
                    value: PsValue::I32(coords.x),
                },
                PsProperty {
                    name: Some("y".to_string()),
                    ref_id: None,
                    value: PsValue::I32(coords.y),
                },
            ],
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl From<Size> for PsObject {
    fn from(size: Size) -> Self {
        PsObject {
            ms: vec![
                PsProperty {
                    name: Some("width".to_string()),
                    ref_id: None,
                    value: PsValue::I32(size.width),
                },
                PsProperty {
                    name: Some("height".to_string()),
                    ref_id: None,
                    value: PsValue::I32(size.height),
                },
            ],
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostDefaultData {
    pub foreground_color: i32,        // Key 0: System.ConsoleColor
    pub background_color: i32,        // Key 1: System.ConsoleColor
    pub cursor_position: Coordinates, // Key 2: System.Management.Automation.Host.Coordinates
    pub window_position: Coordinates, // Key 3: System.Management.Automation.Host.Coordinates
    pub max_physical_cursor_size: i32, // Key 4: System.Int32
    pub window_size: Size,            // Key 5: System.Management.Automation.Host.Size
    pub buffer_size: Size,            // Key 6: System.Management.Automation.Host.Size
    pub max_window_size: Size,        // Key 7: System.Management.Automation.Host.Size
    pub max_physical_window_size: Size, // Key 8: System.Management.Automation.Host.Size
    pub host_name: String,            // Key 9: System.String
}

impl HostDefaultData {
    pub fn default_powershell() -> Self {
        Self {
            foreground_color: 7,  // Gray
            background_color: 0,  // Black
            cursor_position: Coordinates { x: 0, y: 27 },
            window_position: Coordinates { x: 0, y: 0 },
            max_physical_cursor_size: 25,
            window_size: Size { width: 120, height: 30 },
            buffer_size: Size { width: 120, height: 30 },
            max_window_size: Size { width: 120, height: 30 },
            max_physical_window_size: Size { width: 3824, height: 2121 },
            host_name: "PowerShell".to_string(),
        }
    }

    // Convert to the HashMap<PsValue, PsValue> format expected by HostInfo DCT
    pub fn to_dictionary(&self) -> std::collections::HashMap<PsValue, PsValue> {
        let mut map = std::collections::HashMap::new();

        // Key 0: Foreground color
        map.insert(PsValue::I32(0), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.ConsoleColor".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::I32(self.foreground_color) },
            ],
            ..Default::default()
        }));

        // Key 1: Background color
        map.insert(PsValue::I32(1), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.ConsoleColor".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::I32(self.background_color) },
            ],
            ..Default::default()
        }));

        // Key 2: Cursor position
        map.insert(PsValue::I32(2), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.Management.Automation.Host.Coordinates".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::Object(self.cursor_position.clone().into()) },
            ],
            ..Default::default()
        }));

        // Key 3: Window position
        map.insert(PsValue::I32(3), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.Management.Automation.Host.Coordinates".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::Object(self.window_position.clone().into()) },
            ],
            ..Default::default()
        }));

        // Key 4: Max physical cursor size
        map.insert(PsValue::I32(4), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.Int32".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::I32(self.max_physical_cursor_size) },
            ],
            ..Default::default()
        }));

        // Key 5: Window size
        map.insert(PsValue::I32(5), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.Management.Automation.Host.Size".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::Object(self.window_size.clone().into()) },
            ],
            ..Default::default()
        }));

        // Key 6: Buffer size
        map.insert(PsValue::I32(6), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.Management.Automation.Host.Size".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::Object(self.buffer_size.clone().into()) },
            ],
            ..Default::default()
        }));

        // Key 7: Max window size
        map.insert(PsValue::I32(7), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.Management.Automation.Host.Size".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::Object(self.max_window_size.clone().into()) },
            ],
            ..Default::default()
        }));

        // Key 8: Max physical window size
        map.insert(PsValue::I32(8), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.Management.Automation.Host.Size".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::Object(self.max_physical_window_size.clone().into()) },
            ],
            ..Default::default()
        }));

        // Key 9: Host name
        map.insert(PsValue::I32(9), PsValue::Object(PsObject {
            ms: vec![
                PsProperty { name: Some("T".to_string()), ref_id: None, value: PsValue::Str("System.String".to_string()) },
                PsProperty { name: Some("V".to_string()), ref_id: None, value: PsValue::Str(self.host_name.clone()) },
            ],
            ..Default::default()
        }));

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinates_serialization() {
        let coords = Coordinates { x: 10, y: 20 };
        let ps_obj = PsObject::from(coords);
        let xml = ps_obj.to_element().to_string();

        println!("Coordinates XML: {}", xml);

        assert!(xml.contains("<I32 N=\"x\">10</I32>"));
        assert!(xml.contains("<I32 N=\"y\">20</I32>"));
    }

    #[test]
    fn test_size_serialization() {
        let size = Size { width: 120, height: 30 };
        let ps_obj = PsObject::from(size);
        let xml = ps_obj.to_element().to_string();

        println!("Size XML: {}", xml);

        assert!(xml.contains("<I32 N=\"width\">120</I32>"));
        assert!(xml.contains("<I32 N=\"height\">30</I32>"));
    }

    #[test]
    fn test_host_default_data_creation() {
        let host_data = HostDefaultData::default_powershell();
        
        assert_eq!(host_data.foreground_color, 7);
        assert_eq!(host_data.background_color, 0);
        assert_eq!(host_data.cursor_position.x, 0);
        assert_eq!(host_data.cursor_position.y, 27);
        assert_eq!(host_data.host_name, "PowerShell");
        assert_eq!(host_data.buffer_size.width, 120);
        assert_eq!(host_data.max_physical_window_size.width, 3824);
    }

    #[test]
    fn test_host_default_data_to_dictionary() {
        let host_data = HostDefaultData::default_powershell();
        let dict = host_data.to_dictionary();

        println!("Dictionary has {} entries", dict.len());
        
        // Should have all 10 keys (0-9)
        assert_eq!(dict.len(), 10);
        assert!(dict.contains_key(&PsValue::I32(0))); // Foreground color
        assert!(dict.contains_key(&PsValue::I32(1))); // Background color  
        assert!(dict.contains_key(&PsValue::I32(2))); // Cursor position
        assert!(dict.contains_key(&PsValue::I32(9))); // Host name

        // Test a specific entry structure
        if let Some(PsValue::Object(fg_color_obj)) = dict.get(&PsValue::I32(0)) {
            let xml = fg_color_obj.to_element().to_string();
            println!("Foreground color XML: {}", xml);
            assert!(xml.contains("System.ConsoleColor"));
            assert!(xml.contains("<I32 N=\"V\">7</I32>"));
        } else {
            panic!("Foreground color not found or wrong type");
        }
    }

    #[test]
    fn test_host_default_data_dictionary_structure() {
        let host_data = HostDefaultData::default_powershell();
        let dict = host_data.to_dictionary();

        // Convert dictionary to XML to see the full structure
        let dict_obj = PsObject {
            type_names: Some(vec![
                "System.Collections.Hashtable".to_string(),
                "System.Object".to_string(),
            ]),
            dct: dict,
            ..Default::default()
        };

        let xml = dict_obj.to_element().to_string();
        println!("Full dictionary structure:");
        println!("{}", xml);

        // Verify it contains all the expected elements
        assert!(xml.contains("System.ConsoleColor"));
        assert!(xml.contains("System.Management.Automation.Host.Coordinates"));
        assert!(xml.contains("System.Management.Automation.Host.Size"));
        assert!(xml.contains("System.String"));
        assert!(xml.contains("PowerShell"));
    }
}