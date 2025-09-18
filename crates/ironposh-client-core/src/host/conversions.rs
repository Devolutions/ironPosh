use super::{
    error::HostError,
    methods::{BufferCell, Rectangle},
};
use ironposh_psrp::PsValue;

// Spec-compliant Remote Host Method IDs (MS-PSRP §2.2.3.17)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteHostMethodId {
    // Host (read-only)
    GetName = 1,
    GetVersion = 2,
    GetInstanceId = 3,
    GetCurrentCulture = 4,
    GetCurrentUICulture = 5,
    // Host (methods)
    SetShouldExit = 6,
    EnterNestedPrompt = 7,
    ExitNestedPrompt = 8,
    NotifyBeginApplication = 9,
    NotifyEndApplication = 10,
    // UI methods
    ReadLine = 11,
    ReadLineAsSecureString = 12,
    Write1 = 13,
    Write2 = 14,
    WriteLine1 = 15,
    WriteLine2 = 16,
    WriteLine3 = 17,
    WriteErrorLine = 18,
    WriteDebugLine = 19,
    WriteProgress = 20,
    WriteVerboseLine = 21,
    WriteWarningLine = 22,
    Prompt = 23,
    PromptForCredential1 = 24,
    PromptForCredential2 = 25,
    PromptForChoice = 26,
    // RawUI properties/methods
    GetForegroundColor = 27,
    SetForegroundColor = 28,
    GetBackgroundColor = 29,
    SetBackgroundColor = 30,
    GetCursorPosition = 31,
    SetCursorPosition = 32,
    GetWindowPosition = 33,
    SetWindowPosition = 34,
    GetCursorSize = 35,
    SetCursorSize = 36,
    GetBufferSize = 37,
    SetBufferSize = 38,
    GetWindowSize = 39,
    SetWindowSize = 40,
    GetWindowTitle = 41,
    SetWindowTitle = 42,
    GetMaxWindowSize = 43,
    GetMaxPhysicalWindowSize = 44,
    GetKeyAvailable = 45,
    ReadKey = 46,
    FlushInputBuffer = 47,
    SetBufferContents1 = 48,
    SetBufferContents2 = 49,
    GetBufferContents = 50,
    ScrollBufferContents = 51,
    // IHostSupportsInteractiveSession
    PushRunspace = 52,
    PopRunspace = 53,
    GetIsRunspacePushed = 54,
    GetRunspace = 55,
    // IHostSupportsMultipleChoiceSelect
    PromptForChoiceMultipleSelection = 56,
}

impl TryFrom<i32> for RemoteHostMethodId {
    type Error = HostError;
    
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        use RemoteHostMethodId::*;
        Ok(match v {
            1 => GetName, 2 => GetVersion, 3 => GetInstanceId, 4 => GetCurrentCulture,
            5 => GetCurrentUICulture, 6 => SetShouldExit, 7 => EnterNestedPrompt,
            8 => ExitNestedPrompt, 9 => NotifyBeginApplication, 10 => NotifyEndApplication,
            11 => ReadLine, 12 => ReadLineAsSecureString, 13 => Write1, 14 => Write2,
            15 => WriteLine1, 16 => WriteLine2, 17 => WriteLine3, 18 => WriteErrorLine,
            19 => WriteDebugLine, 20 => WriteProgress, 21 => WriteVerboseLine,
            22 => WriteWarningLine, 23 => Prompt, 24 => PromptForCredential1,
            25 => PromptForCredential2, 26 => PromptForChoice, 27 => GetForegroundColor,
            28 => SetForegroundColor, 29 => GetBackgroundColor, 30 => SetBackgroundColor,
            31 => GetCursorPosition, 32 => SetCursorPosition, 33 => GetWindowPosition,
            34 => SetWindowPosition, 35 => GetCursorSize, 36 => SetCursorSize,
            37 => GetBufferSize, 38 => SetBufferSize, 39 => GetWindowSize, 40 => SetWindowSize,
            41 => GetWindowTitle, 42 => SetWindowTitle, 43 => GetMaxWindowSize,
            44 => GetMaxPhysicalWindowSize, 45 => GetKeyAvailable, 46 => ReadKey,
            47 => FlushInputBuffer, 48 => SetBufferContents1, 49 => SetBufferContents2,
            50 => GetBufferContents, 51 => ScrollBufferContents, 52 => PushRunspace,
            53 => PopRunspace, 54 => GetIsRunspacePushed, 55 => GetRunspace,
            56 => PromptForChoiceMultipleSelection,
            _ => return Err(HostError::NotImplemented),
        })
    }
}

// Response gating per spec - only methods that return values should send responses
pub fn should_send_host_response(id: RemoteHostMethodId) -> bool {
    use RemoteHostMethodId::*;
    matches!(id,
        // Methods that DO return a value
        GetName | GetVersion | GetInstanceId | GetCurrentCulture | GetCurrentUICulture |
        ReadLine | ReadLineAsSecureString | Prompt | PromptForCredential1 | PromptForCredential2 |
        PromptForChoice |
        GetForegroundColor | GetBackgroundColor | GetCursorPosition | GetWindowPosition |
        GetCursorSize | GetBufferSize | GetWindowSize | GetWindowTitle | GetMaxWindowSize |
        GetMaxPhysicalWindowSize | GetKeyAvailable | ReadKey | GetBufferContents |
        GetIsRunspacePushed | GetRunspace | PromptForChoiceMultipleSelection
    )
}

// PsValue extension methods for complex type parsing
trait PsValueExt {
    fn as_coordinates(&self) -> Option<(i32, i32)>;
    fn as_size(&self) -> Option<(i32, i32)>;
    fn as_rectangle(&self) -> Option<Rectangle>;
    fn as_buffer_cell(&self) -> Option<BufferCell>;
    fn as_buffer_cell_array(&self) -> Option<Vec<BufferCell>>;
}

impl PsValueExt for PsValue {
    fn as_coordinates(&self) -> Option<(i32, i32)> {
        if let PsValue::Object(o) = self {
            let x = o.extended_properties.get("x")
                .or_else(|| o.extended_properties.get("X"))
                .and_then(|prop| prop.value.as_i32())?;
            let y = o.extended_properties.get("y")
                .or_else(|| o.extended_properties.get("Y"))
                .and_then(|prop| prop.value.as_i32())?;
            return Some((x, y));
        }
        None
    }

    fn as_size(&self) -> Option<(i32, i32)> {
        if let PsValue::Object(o) = self {
            // Try flat structure first
            if let (Some(w_prop), Some(h_prop)) = (
                o.extended_properties.get("width").or_else(|| o.extended_properties.get("Width")),
                o.extended_properties.get("height").or_else(|| o.extended_properties.get("Height"))
            ) {
                let w = w_prop.value.as_i32()?;
                let h = h_prop.value.as_i32()?;
                return Some((w, h));
            }
            
            // Try nested V.{width,height} structure per spec
            if let Some(v_prop) = o.extended_properties.get("V") {
                if let PsValue::Object(vobj) = &v_prop.value {
                    let w = vobj.extended_properties.get("width")
                        .or_else(|| vobj.extended_properties.get("Width"))
                        .and_then(|prop| prop.value.as_i32())?;
                    let h = vobj.extended_properties.get("height")
                        .or_else(|| vobj.extended_properties.get("Height"))
                        .and_then(|prop| prop.value.as_i32())?;
                    return Some((w, h));
                }
            }
        }
        None
    }

    fn as_rectangle(&self) -> Option<Rectangle> {
        if let PsValue::Object(o) = self {
            let left = o.extended_properties.get("Left")
                .or_else(|| o.extended_properties.get("left"))
                .and_then(|prop| prop.value.as_i32())?;
            let top = o.extended_properties.get("Top")
                .or_else(|| o.extended_properties.get("top"))
                .and_then(|prop| prop.value.as_i32())?;
            let right = o.extended_properties.get("Right")
                .or_else(|| o.extended_properties.get("right"))
                .and_then(|prop| prop.value.as_i32())?;
            let bottom = o.extended_properties.get("Bottom")
                .or_else(|| o.extended_properties.get("bottom"))
                .and_then(|prop| prop.value.as_i32())?;
            return Some(Rectangle { left, top, right, bottom });
        }
        None
    }

    fn as_buffer_cell(&self) -> Option<BufferCell> {
        if let PsValue::Object(o) = self {
            let character = o.extended_properties.get("Character")
                .or_else(|| o.extended_properties.get("character"))
                .and_then(|prop| prop.value.as_string())
                .and_then(|s| s.chars().next())?;
            
            let foreground = o.extended_properties.get("ForegroundColor")
                .or_else(|| o.extended_properties.get("foregroundColor"))
                .and_then(|prop| prop.value.as_i32())?;
            
            let background = o.extended_properties.get("BackgroundColor")
                .or_else(|| o.extended_properties.get("backgroundColor"))
                .and_then(|prop| prop.value.as_i32())?;
            
            let flags = o.extended_properties.get("BufferCellType")
                .or_else(|| o.extended_properties.get("bufferCellType"))
                .and_then(|prop| prop.value.as_i32())
                .unwrap_or(0);
            
            return Some(BufferCell { character, foreground, background, flags });
        }
        None
    }

    fn as_buffer_cell_array(&self) -> Option<Vec<BufferCell>> {
        match self {
            PsValue::Object(o) => {
                match &o.content {
                    ironposh_psrp::ComplexObjectContent::Container(
                        ironposh_psrp::Container::List(items)
                    ) => {
                        let mut cells = Vec::with_capacity(items.len());
                        for item in items {
                            cells.push(item.as_buffer_cell()?);
                        }
                        return Some(cells);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        None
    }
}
