use ironposh_macros::PsEnum;

/// Host method identifier (MS-PSRP §2.2.3.17) — the `mi` field of host messages.
///
/// Serializes as a `RemoteHostMethodId` enum `<Obj>` (type-name chain +
/// `<ToString>` of the method name + `<I32>` id), all macro-derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PsEnum)]
#[ps(
    repr = "object",
    type_names(
        "System.Management.Automation.Remoting.RemoteHostMethodId",
        "System.Enum",
        "System.ValueType",
        "System.Object"
    )
)]
pub enum RemoteHostMethodId {
    GetName = 1,
    GetVersion = 2,
    GetInstanceId = 3,
    GetCurrentCulture = 4,
    GetCurrentUICulture = 5,
    SetShouldExit = 6,
    EnterNestedPrompt = 7,
    ExitNestedPrompt = 8,
    NotifyBeginApplication = 9,
    NotifyEndApplication = 10,
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
    PushRunspace = 52,
    PopRunspace = 53,
    GetIsRunspacePushed = 54,
    GetRunspace = 55,
    PromptForChoiceMultipleSelection = 56,
}

impl RemoteHostMethodId {
    /// The numeric method identifier.
    pub fn id(self) -> i32 {
        self as i32
    }

    /// Map a numeric method id to its variant (per MS-PSRP §2.2.3.17).
    pub fn from_id(id: i32) -> Option<Self> {
        Self::__ps_from_discriminant(id)
    }
}
