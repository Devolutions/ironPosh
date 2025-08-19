use super::{HostError, HostResult};
use protocol_powershell_remoting::{ComplexObject, PsValue};

/// Defines the lowest-level user interface functions that an interactive
/// application hosting a Runspace can choose to implement if it wants
/// to support any cmdlet that does character-mode interaction with the user.
/// Corresponds to System.Management.Automation.Host.PSHostRawUserInterface
pub trait PSHostRawUserInterface {
    /// Returns the foreground color of the hosting application
    /// MI: 27
    fn get_foreground_color(&self) -> HostResult<i32>; // ConsoleColor

    /// Sets the foreground color of the hosting application
    /// MI: 28
    fn set_foreground_color(&mut self, color: i32) -> HostResult<()>;

    /// Returns the background color of the hosting application
    /// MI: 29
    fn get_background_color(&self) -> HostResult<i32>; // ConsoleColor

    /// Sets the background color of the hosting application
    /// MI: 30
    fn set_background_color(&mut self, color: i32) -> HostResult<()>;

    /// Returns the current cursor position in the hosting application
    /// MI: 31
    fn get_cursor_position(&self) -> HostResult<(i32, i32)>; // Coordinates

    /// Sets the current cursor position in the hosting application
    /// MI: 32
    fn set_cursor_position(&mut self, x: i32, y: i32) -> HostResult<()>;

    /// Returns the position of the view window relative to the screen buffer
    /// MI: 33
    fn get_window_position(&self) -> HostResult<(i32, i32)>; // Coordinates

    /// Sets the position of the view window relative to the screen buffer
    /// MI: 34
    fn set_window_position(&mut self, x: i32, y: i32) -> HostResult<()>;

    /// Returns the cursor size as a percentage
    /// MI: 35
    fn get_cursor_size(&self) -> HostResult<i32>;

    /// Sets the cursor size based on the percentage value specified
    /// MI: 36
    fn set_cursor_size(&mut self, percentage: i32) -> HostResult<()>;

    /// Returns the current size of the screen buffer, measured in character cells
    /// MI: 37
    fn get_buffer_size(&self) -> HostResult<(i32, i32)>; // Size

    /// Sets the size of the screen buffer with the specified size in character cells
    /// MI: 38
    fn set_buffer_size(&mut self, width: i32, height: i32) -> HostResult<()>;

    /// Returns the current view window size
    /// MI: 39
    fn get_window_size(&self) -> HostResult<(i32, i32)>; // Size

    /// Sets the view window size based on the size specified
    /// MI: 40
    fn set_window_size(&mut self, width: i32, height: i32) -> HostResult<()>;

    /// Returns the title of the hosting application's window
    /// MI: 41
    fn get_window_title(&self) -> HostResult<String>;

    /// Sets the window title
    /// MI: 42
    fn set_window_title(&mut self, title: &str) -> HostResult<()>;

    /// Returns the maximum window size possible for the current buffer,
    /// current font, and current display hardware
    /// MI: 43
    fn get_max_window_size(&self) -> HostResult<(i32, i32)>; // Size

    /// Returns the maximum window size possible for the current font and
    /// current display hardware, ignoring the current buffer size
    /// MI: 44
    fn get_max_physical_window_size(&self) -> HostResult<(i32, i32)>; // Size

    /// Examines if a keystroke is waiting on the input, returning true if so and false otherwise
    /// MI: 45
    fn get_key_available(&self) -> HostResult<bool>;

    /// Reads a key stroke from the keyboard, blocking until a key is typed
    /// MI: 46
    fn read_key(&mut self, options: i32) -> HostResult<ComplexObject>; // KeyInfo

    /// Resets the keyboard input buffer
    /// MI: 47
    fn flush_input_buffer(&mut self) -> HostResult<()>;

    /// Copies the specified buffer cell array into the screen buffer at the specified coordinates
    /// MI: 48
    fn set_buffer_contents_array(
        &mut self,
        origin_x: i32,
        origin_y: i32,
        contents: &[ComplexObject], // BufferCell array
    ) -> HostResult<()>;

    /// Copies the specified buffer cell into all the cells within the specified rectangle
    /// MI: 49
    fn set_buffer_contents_fill(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        fill: &ComplexObject, // BufferCell
    ) -> HostResult<()>;

    /// Returns the contents in a specified rectangular region of the hosting application's window
    /// MI: 50
    fn get_buffer_contents(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> HostResult<Vec<ComplexObject>>; // Array of BufferCell

    /// Scrolls a region on the screen buffer
    /// MI: 51
    fn scroll_buffer_contents(
        &mut self,
        source_left: i32,
        source_top: i32,
        source_right: i32,
        source_bottom: i32,
        destination_x: i32,
        destination_y: i32,
        clip_left: i32,
        clip_top: i32,
        clip_right: i32,
        clip_bottom: i32,
        fill: &ComplexObject, // BufferCell
    ) -> HostResult<()>;

    /// Runs a raw UI method call based on the method identifier and parameters
    fn run_method(
        &mut self,
        method_id: i32,
        _method_name: &str,
        parameters: &[PsValue],
    ) -> HostResult<Option<PsValue>> {
        match method_id {
            27 => {
                let color = self.get_foreground_color()?;
                Ok(Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::I32(color),
                )))
            }
            28 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::I32(color),
                )) = parameters.first()
                {
                    self.set_foreground_color(*color)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            29 => {
                let color = self.get_background_color()?;
                Ok(Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::I32(color),
                )))
            }
            30 => {
                if let Some(PsValue::Primitive(
                    protocol_powershell_remoting::PsPrimitiveValue::I32(color),
                )) = parameters.first()
                {
                    self.set_background_color(*color)?;
                    Ok(None)
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            31 => {
                let (x, y) = self.get_cursor_position()?;
                // Return as Coordinates complex object
                let mut props = std::collections::BTreeMap::new();
                props.insert(
                    "x".to_string(),
                    protocol_powershell_remoting::PsProperty {
                        name: "x".to_string(),
                        value: PsValue::Primitive(
                            protocol_powershell_remoting::PsPrimitiveValue::I32(x),
                        ),
                    },
                );
                props.insert(
                    "y".to_string(),
                    protocol_powershell_remoting::PsProperty {
                        name: "y".to_string(),
                        value: PsValue::Primitive(
                            protocol_powershell_remoting::PsPrimitiveValue::I32(y),
                        ),
                    },
                );
                let coords = ComplexObject {
                    type_def: None,
                    to_string: None,
                    content: protocol_powershell_remoting::ComplexObjectContent::Standard,
                    adapted_properties: std::collections::BTreeMap::new(),
                    extended_properties: props,
                };
                Ok(Some(PsValue::Object(coords)))
            }
            32 => {
                // Extract coordinates from complex object parameter
                if let Some(PsValue::Object(coords)) = parameters.first() {
                    let x = coords
                        .extended_properties
                        .get("x")
                        .and_then(|p| match &p.value {
                            PsValue::Primitive(
                                protocol_powershell_remoting::PsPrimitiveValue::I32(x),
                            ) => Some(*x),
                            _ => None,
                        });
                    let y = coords
                        .extended_properties
                        .get("y")
                        .and_then(|p| match &p.value {
                            PsValue::Primitive(
                                protocol_powershell_remoting::PsPrimitiveValue::I32(y),
                            ) => Some(*y),
                            _ => None,
                        });

                    if let (Some(x), Some(y)) = (x, y) {
                        self.set_cursor_position(x, y)?;
                        Ok(None)
                    } else {
                        Err(HostError::InvalidParameters)
                    }
                } else {
                    Err(HostError::InvalidParameters)
                }
            }
            33..=51 => {
                // Other raw UI methods - implement as needed
                Err(HostError::NotImplemented)
            }
            _ => Err(HostError::NotImplemented),
        }
    }
}
