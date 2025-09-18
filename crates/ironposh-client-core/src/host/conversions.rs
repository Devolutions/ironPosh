use super::{
    error::HostError,
    methods::{
        HostCallMethodWithParams, HostMethodParams, RawUIMethodParams, UIMethodParams, 
        BufferCell, Rectangle,
    },
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

/// Convert HostCallRequest to HostCallMethodWithParams based on method_id and parameters
impl TryFrom<&super::HostCallRequest> for HostCallMethodWithParams {
    type Error = HostError;

    fn try_from(call: &super::HostCallRequest) -> Result<Self, Self::Error> {
        let id = RemoteHostMethodId::try_from(call.method_id)?;

        use HostCallMethodWithParams::*;
        use HostMethodParams as HM;
        use UIMethodParams as UI;
        use RawUIMethodParams as RUI;

        let p = &call.parameters;

        Ok(match id {
            // Host methods (1-10)
            RemoteHostMethodId::GetName => HostMethod(HM::GetName),
            RemoteHostMethodId::GetVersion => HostMethod(HM::GetVersion),
            RemoteHostMethodId::GetInstanceId => HostMethod(HM::GetInstanceId),
            RemoteHostMethodId::GetCurrentCulture => HostMethod(HM::GetCurrentCulture),
            RemoteHostMethodId::GetCurrentUICulture => HostMethod(HM::GetCurrentUICulture),
            RemoteHostMethodId::SetShouldExit => {
                let exit_code = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                HostMethod(HM::SetShouldExit(exit_code))
            }
            RemoteHostMethodId::EnterNestedPrompt => HostMethod(HM::EnterNestedPrompt),
            RemoteHostMethodId::ExitNestedPrompt => HostMethod(HM::ExitNestedPrompt),
            RemoteHostMethodId::NotifyBeginApplication => HostMethod(HM::NotifyBeginApplication),
            RemoteHostMethodId::NotifyEndApplication => HostMethod(HM::NotifyEndApplication),

            // UI methods (11-26)
            RemoteHostMethodId::ReadLine => UIMethod(UI::ReadLine),
            RemoteHostMethodId::ReadLineAsSecureString => UIMethod(UI::ReadLineAsSecureString),
            RemoteHostMethodId::Write1 => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::Write { text, fg: None, bg: None, newline: false })
            }
            RemoteHostMethodId::Write2 => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let fg = p.get(1).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                let bg = p.get(2).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::Write { text, fg: Some(fg), bg: Some(bg), newline: false })
            }
            RemoteHostMethodId::WriteLine1 => UIMethod(UI::Write { text: String::new(), fg: None, bg: None, newline: true }),
            RemoteHostMethodId::WriteLine2 => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::Write { text, fg: None, bg: None, newline: true })
            }
            RemoteHostMethodId::WriteLine3 => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let fg = p.get(1).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                let bg = p.get(2).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::Write { text, fg: Some(fg), bg: Some(bg), newline: true })
            }
            RemoteHostMethodId::WriteErrorLine => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::WriteErrorLine(text))
            }
            RemoteHostMethodId::WriteDebugLine => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::WriteDebugLine(text))
            }
            RemoteHostMethodId::WriteProgress => {
                // Accept ProgressRecord object - keep as PsValue to preserve structure
                let progress = p.get(1).or_else(|| p.get(0)).cloned().ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::WriteProgress(progress))
            }
            RemoteHostMethodId::WriteVerboseLine => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::WriteVerboseLine(text))
            }
            RemoteHostMethodId::WriteWarningLine => {
                let text = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::WriteWarningLine(text))
            }
            RemoteHostMethodId::Prompt => {
                let caption = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let descriptions = p.get(1).cloned().ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::Prompt { caption, descriptions })
            }
            RemoteHostMethodId::PromptForCredential1 | RemoteHostMethodId::PromptForCredential2 => {
                let caption = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let message = p.get(1).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let user_name = p.get(2).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let target_name = p.get(3).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::PromptForCredential { 
                    caption, message, user_name, target_name,
                    extra: p.get(4).cloned() // Method 25 has extra params
                })
            }
            RemoteHostMethodId::PromptForChoice => {
                let caption = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let message = p.get(1).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                let choices = p.get(2).cloned().ok_or(HostError::InvalidParameters)?;
                let default_choice = p.get(3).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                UIMethod(UI::PromptForChoice { caption, message, choices, default_choice })
            }

            // RawUI methods (27-51) - FIXED MAPPING!
            RemoteHostMethodId::GetForegroundColor => RawUIMethod(RUI::GetForegroundColor),
            RemoteHostMethodId::SetForegroundColor => {
                let color = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::SetForegroundColor(color))
            }
            RemoteHostMethodId::GetBackgroundColor => RawUIMethod(RUI::GetBackgroundColor),
            RemoteHostMethodId::SetBackgroundColor => {
                let color = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::SetBackgroundColor(color))
            }
            RemoteHostMethodId::GetCursorPosition => RawUIMethod(RUI::GetCursorPosition),
            RemoteHostMethodId::SetCursorPosition => {
                // Accept Coordinates object or (x,y) pair
                if let Some((x, y)) = p.get(0).and_then(|v| v.as_coordinates()) {
                    RawUIMethod(RUI::SetCursorPosition(x, y))
                } else {
                    let x = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    let y = p.get(1).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    RawUIMethod(RUI::SetCursorPosition(x, y))
                }
            }
            RemoteHostMethodId::GetWindowPosition => RawUIMethod(RUI::GetWindowPosition),
            RemoteHostMethodId::SetWindowPosition => {
                if let Some((x, y)) = p.get(0).and_then(|v| v.as_coordinates()) {
                    RawUIMethod(RUI::SetWindowPosition(x, y))
                } else {
                    let x = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    let y = p.get(1).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    RawUIMethod(RUI::SetWindowPosition(x, y))
                }
            }
            RemoteHostMethodId::GetCursorSize => RawUIMethod(RUI::GetCursorSize),
            RemoteHostMethodId::SetCursorSize => {
                let pct = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::SetCursorSize(pct))
            }
            RemoteHostMethodId::GetBufferSize => RawUIMethod(RUI::GetBufferSize),
            RemoteHostMethodId::SetBufferSize => {
                if let Some((width, height)) = p.get(0).and_then(|v| v.as_size()) {
                    RawUIMethod(RUI::SetBufferSize(width, height))
                } else {
                    let width = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    let height = p.get(1).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    RawUIMethod(RUI::SetBufferSize(width, height))
                }
            }
            RemoteHostMethodId::GetWindowSize => RawUIMethod(RUI::GetWindowSize),
            RemoteHostMethodId::SetWindowSize => {
                if let Some((width, height)) = p.get(0).and_then(|v| v.as_size()) {
                    RawUIMethod(RUI::SetWindowSize(width, height))
                } else {
                    let width = p.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    let height = p.get(1).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
                    RawUIMethod(RUI::SetWindowSize(width, height))
                }
            }
            RemoteHostMethodId::GetWindowTitle => RawUIMethod(RUI::GetWindowTitle),
            RemoteHostMethodId::SetWindowTitle => {
                let title = p.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::SetWindowTitle(title))
            }
            RemoteHostMethodId::GetMaxWindowSize => RawUIMethod(RUI::GetMaxWindowSize),
            RemoteHostMethodId::GetMaxPhysicalWindowSize => RawUIMethod(RUI::GetMaxPhysicalWindowSize),
            RemoteHostMethodId::GetKeyAvailable => RawUIMethod(RUI::GetKeyAvailable),
            RemoteHostMethodId::ReadKey => {
                let opts = p.get(0).and_then(|v| v.as_i32()).unwrap_or(0);
                RawUIMethod(RUI::ReadKey(opts))
            }
            RemoteHostMethodId::FlushInputBuffer => RawUIMethod(RUI::FlushInputBuffer),
            RemoteHostMethodId::SetBufferContents1 => {
                // params: BufferCell[] at coords (Coordinates)
                let (x, y) = p.get(0).and_then(|v| v.as_coordinates()).ok_or(HostError::InvalidParameters)?;
                let cells = p.get(1).and_then(|v| v.as_buffer_cell_array()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::SetBufferContentsArray { x, y, cells })
            }
            RemoteHostMethodId::SetBufferContents2 => {
                // params: BufferCell + Rectangle
                let cell = p.get(0).and_then(|v| v.as_buffer_cell()).ok_or(HostError::InvalidParameters)?;
                let rect = p.get(1).and_then(|v| v.as_rectangle()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::SetBufferContentsRect { cell, rect })
            }
            RemoteHostMethodId::GetBufferContents => {
                let rect = p.get(0).and_then(|v| v.as_rectangle()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::GetBufferContents { rect })
            }
            RemoteHostMethodId::ScrollBufferContents => {
                // params: sourceRect, dest, clip, fill
                let src = p.get(0).and_then(|v| v.as_rectangle()).ok_or(HostError::InvalidParameters)?;
                let (dest_x, dest_y) = p.get(1).and_then(|v| v.as_coordinates()).ok_or(HostError::InvalidParameters)?;
                let clip = p.get(2).and_then(|v| v.as_rectangle()).ok_or(HostError::InvalidParameters)?;
                let fill = p.get(3).and_then(|v| v.as_buffer_cell()).ok_or(HostError::InvalidParameters)?;
                RawUIMethod(RUI::ScrollBufferContents { src, dest_x, dest_y, clip, fill })
            }

            // Interactive session methods (52-55)
            RemoteHostMethodId::PushRunspace => HostMethod(HM::PushRunspace),
            RemoteHostMethodId::PopRunspace => HostMethod(HM::PopRunspace),
            RemoteHostMethodId::GetIsRunspacePushed => HostMethod(HM::GetIsRunspacePushed),
            RemoteHostMethodId::GetRunspace => HostMethod(HM::GetRunspace),

            // Multiple choice method (56)
            RemoteHostMethodId::PromptForChoiceMultipleSelection => UIMethod(UI::PromptForChoiceMultipleSelection),
        })
    }
}
