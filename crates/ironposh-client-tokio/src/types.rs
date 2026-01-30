use ironposh_terminal::TerminalOp;
use tokio::sync::oneshot;

use ironposh_client_core::host::{
    BufferCell, ChoiceDescription, Coordinates, FieldDescription, KeyInfo, PSCredential, Rectangle,
};
use ironposh_psrp::PsValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ReplControl {
    EnterNestedPrompt,
    ExitNestedPrompt,
    ShouldExit(i32),
}

#[derive(Debug)]
pub enum HostUiRequest {
    ReadLine,
    ReadLineAsSecureString,
    Prompt {
        caption: String,
        message: String,
        fields: Vec<FieldDescription>,
    },
    PromptForChoice {
        caption: String,
        message: String,
        choices: Vec<ChoiceDescription>,
        default_choice: i32,
    },
    PromptForChoiceMultipleSelection {
        caption: String,
        message: String,
        choices: Vec<ChoiceDescription>,
        default_choices: Vec<i32>,
    },
    PromptForCredential1 {
        caption: String,
        message: String,
        user_name: String,
        target_name: String,
    },
    PromptForCredential2 {
        caption: String,
        message: String,
        user_name: String,
        target_name: String,
        allowed_credential_types: i32,
        options: i32,
    },
    ReadKey {
        options: i32,
    },
    GetKeyAvailable,
    FlushInputBuffer,
    GetCursorPosition,
    GetBufferContents {
        rect: Rectangle,
    },
}

#[derive(Debug)]
pub enum HostUiResponse {
    Line(String),
    SecureBytes(Vec<u8>),
    PromptResult(HashMap<String, PsValue>),
    Choice(i32),
    ChoiceMulti(Vec<i32>),
    Credential(PSCredential),
    KeyInfo(KeyInfo),
    Bool(bool),
    Unit,
    CursorPosition(Coordinates),
    BufferContents(Vec<Vec<BufferCell>>),
}

/// UI operations for the async UI handler
#[derive(Debug)]
pub enum TerminalOperation {
    /// Apply terminal operations (cursor move, clear, fill, etc.)
    Apply(Vec<TerminalOp>),
    /// Print plain text lines
    Print(String),
    /// Write plain text, optionally adding a newline.
    Write { text: String, newline: bool },
    /// Set the host terminal window title (best-effort).
    SetWindowTitle { title: String },
    /// Request input from user with given prompt
    RequestInput { prompt: String },
    /// Check input for interrupt (Ctrl-C)
    CheckInterrupt,
    /// A synchronous UI request that needs a response (used by HostCalls).
    HostRequest {
        request: HostUiRequest,
        respond_to: oneshot::Sender<HostUiResponse>,
    },
}
