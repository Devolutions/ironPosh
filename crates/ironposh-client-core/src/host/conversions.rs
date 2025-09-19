use super::error::HostError;

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
            1 => GetName,
            2 => GetVersion,
            3 => GetInstanceId,
            4 => GetCurrentCulture,
            5 => GetCurrentUICulture,
            6 => SetShouldExit,
            7 => EnterNestedPrompt,
            8 => ExitNestedPrompt,
            9 => NotifyBeginApplication,
            10 => NotifyEndApplication,
            11 => ReadLine,
            12 => ReadLineAsSecureString,
            13 => Write1,
            14 => Write2,
            15 => WriteLine1,
            16 => WriteLine2,
            17 => WriteLine3,
            18 => WriteErrorLine,
            19 => WriteDebugLine,
            20 => WriteProgress,
            21 => WriteVerboseLine,
            22 => WriteWarningLine,
            23 => Prompt,
            24 => PromptForCredential1,
            25 => PromptForCredential2,
            26 => PromptForChoice,
            27 => GetForegroundColor,
            28 => SetForegroundColor,
            29 => GetBackgroundColor,
            30 => SetBackgroundColor,
            31 => GetCursorPosition,
            32 => SetCursorPosition,
            33 => GetWindowPosition,
            34 => SetWindowPosition,
            35 => GetCursorSize,
            36 => SetCursorSize,
            37 => GetBufferSize,
            38 => SetBufferSize,
            39 => GetWindowSize,
            40 => SetWindowSize,
            41 => GetWindowTitle,
            42 => SetWindowTitle,
            43 => GetMaxWindowSize,
            44 => GetMaxPhysicalWindowSize,
            45 => GetKeyAvailable,
            46 => ReadKey,
            47 => FlushInputBuffer,
            48 => SetBufferContents1,
            49 => SetBufferContents2,
            50 => GetBufferContents,
            51 => ScrollBufferContents,
            52 => PushRunspace,
            53 => PopRunspace,
            54 => GetIsRunspacePushed,
            55 => GetRunspace,
            56 => PromptForChoiceMultipleSelection,
            _ => return Err(HostError::NotImplemented),
        })
    }
}

// Response gating per spec - only methods that return values should send responses
pub fn should_send_host_response(id: RemoteHostMethodId) -> bool {
    use RemoteHostMethodId::*;
    matches!(
        id,
        // Methods that DO return a value
        GetName
            | GetVersion
            | GetInstanceId
            | GetCurrentCulture
            | GetCurrentUICulture
            | ReadLine
            | ReadLineAsSecureString
            | Prompt
            | PromptForCredential1
            | PromptForCredential2
            | PromptForChoice
            | GetForegroundColor
            | GetBackgroundColor
            | GetCursorPosition
            | GetWindowPosition
            | GetCursorSize
            | GetBufferSize
            | GetWindowSize
            | GetWindowTitle
            | GetMaxWindowSize
            | GetMaxPhysicalWindowSize
            | GetKeyAvailable
            | ReadKey
            | GetBufferContents
            | GetIsRunspacePushed
            | GetRunspace
            | PromptForChoiceMultipleSelection
    )
}
