use super::command::Command;
use ironposh_macros::{PsDeserialize, PsSerialize};

/// A PowerShell pipeline (MS-PSRP §2.2.3.11): a list of commands plus flags.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
pub struct PowerShellPipeline {
    #[builder(default = false)]
    #[ps(name = "IsNested")]
    pub is_nested: bool,
    #[builder(setter(into))]
    #[ps(name = "Cmds")]
    pub cmds: Vec<Command>,
    #[builder(default)]
    #[ps(name = "History", with = "history_conv", default)]
    pub history: String,
    #[builder(default = false)]
    #[ps(name = "RedirectShellErrorOutputPipe")]
    pub redirect_shell_error_output_pipe: bool,
}

/// `#[ps(with)]`: History is emitted as `Nil` when empty, a string otherwise.
mod history_conv {
    use ironposh_psrp::PowerShellRemotingError;
    use ironposh_psrp::ps_value::{PsPrimitiveValue, PsValue};

    pub fn to_ps_value(value: &str) -> PsValue {
        if value.is_empty() {
            PsValue::Primitive(PsPrimitiveValue::Nil)
        } else {
            PsValue::Primitive(PsPrimitiveValue::Str(value.to_string()))
        }
    }

    #[allow(clippy::unnecessary_wraps)] // signature fixed by #[ps(with)]
    pub fn from_ps_value(value: &PsValue) -> Result<String, PowerShellRemotingError> {
        Ok(match value {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => s.clone(),
            _ => String::new(),
        })
    }
}
