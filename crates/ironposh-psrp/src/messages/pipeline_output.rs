use std::fmt::Display;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::PsValue;
use crate::{
    MessageType, PowerShellRemotingError, PowerShellRemotingMessage, PsObjectWithType,
    PsPrimitiveValue,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PipelineOutput {
    pub data: PsValue, // the actual output object (primitive or complex)
}

impl PipelineOutput {
    pub fn assume_primitive_string(&self) -> Result<&String, PowerShellRemotingError> {
        match &self.data {
            PsValue::Primitive(PsPrimitiveValue::Str(s)) => Ok(s),
            _ => Err(PowerShellRemotingError::OutputFormattingError(
                "Pipeline output is not a string",
            )),
        }
    }

    pub fn format_as_displyable_string(&self) -> Result<String, PowerShellRemotingError> {
        let Some(output_str) = self.data.as_string() else {
            return Err(PowerShellRemotingError::OutputFormattingError(
                "Pipeline output is not a string",
            ));
        };

        decode_escaped_ps_string(&output_str)
    }
}

impl From<PsValue> for PipelineOutput {
    fn from(v: PsValue) -> Self {
        Self { data: v }
    }
}

impl PsObjectWithType for PipelineOutput {
    fn message_type(&self) -> MessageType {
        MessageType::PipelineOutput
    }

    // IMPORTANT: return the inner PsValue directly; no extra wrapper.
    fn to_ps_object(&self) -> PsValue {
        self.data.clone()
    }
}

impl Display for PipelineOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl TryFrom<&PowerShellRemotingMessage> for PipelineOutput {
    type Error = PowerShellRemotingError;

    fn try_from(msg: &PowerShellRemotingMessage) -> Result<Self, Self::Error> {
        if msg.message_type != MessageType::PipelineOutput {
            return Err(PowerShellRemotingError::InvalidMessage(
                "not a PipelineOutput message".into(),
            ));
        }
        Ok(PipelineOutput {
            data: msg.parse_ps_message()?,
        })
    }
}

fn decode_escaped_ps_string(input: &str) -> Result<String, PowerShellRemotingError> {
    if input.is_empty() {
        return Ok(String::new());
    }

    // Split with capturing parentheses to include the separator in the resulting array
    let regex = Regex::new(r"(_x[0-9A-F]{4}_)")
        .map_err(|_| PowerShellRemotingError::OutputFormattingError("Regex error"))?;
    let parts: Vec<&str> = regex.split(input).collect();

    if parts.len() <= 1 {
        return Ok(input.to_string());
    }

    let mut result = String::new();
    let mut high_surrogate: Option<u16> = None;

    // We need to manually handle the split parts and captures
    let mut current_pos = 0;
    for captures in regex.find_iter(input) {
        // Add the text before the match
        if captures.start() > current_pos {
            result.push_str(&input[current_pos..captures.start()]);
            high_surrogate = None;
        }

        // Process the escaped sequence
        let escaped = captures.as_str();
        if let Some(hex_str) = escaped.strip_prefix("_x").and_then(|s| s.strip_suffix("_")) {
            match u16::from_str_radix(hex_str, 16) {
                Ok(code_unit) => {
                    if let Some(high) = high_surrogate {
                        // We have a high surrogate from before, try to form a surrogate pair
                        if (0xDC00..=0xDFFF).contains(&code_unit) {
                            // This is a low surrogate, form the pair
                            let code_point = 0x10000
                                + ((high as u32 - 0xD800) << 10)
                                + (code_unit as u32 - 0xDC00);
                            if let Some(ch) = char::from_u32(code_point) {
                                result.push(ch);
                            } else {
                                // Invalid code point, add the escaped sequence as-is
                                result.push_str(escaped);
                            }
                            high_surrogate = None;
                        } else {
                            // Not a low surrogate, add the previous high surrogate as-is and process this one
                            result.push_str("_x");
                            result.push_str(&format!("{high:04X}"));
                            result.push('_');

                            if (0xD800..=0xDBFF).contains(&code_unit) {
                                high_surrogate = Some(code_unit);
                            } else {
                                if let Some(ch) = char::from_u32(code_unit as u32) {
                                    result.push(ch);
                                } else {
                                    result.push_str(escaped);
                                }
                                high_surrogate = None;
                            }
                        }
                    } else if (0xD800..=0xDBFF).contains(&code_unit) {
                        // High surrogate, save it for the next iteration
                        high_surrogate = Some(code_unit);
                    } else {
                        // Regular character or low surrogate without high surrogate
                        if let Some(ch) = char::from_u32(code_unit as u32) {
                            result.push(ch);
                        } else {
                            // Invalid character, add the escaped sequence as-is
                            result.push_str(escaped);
                        }
                        high_surrogate = None;
                    }
                }
                Err(_) => {
                    // Invalid hex, add the escaped sequence as-is
                    result.push_str(escaped);
                    high_surrogate = None;
                }
            }
        } else {
            // Not a valid escape sequence, add as-is
            result.push_str(escaped);
            high_surrogate = None;
        }

        current_pos = captures.end();
    }

    // Add any remaining text after the last match
    if current_pos < input.len() {
        result.push_str(&input[current_pos..]);
    }

    // If we have an unmatched high surrogate at the end, add it as-is
    if let Some(high) = high_surrogate {
        result.push_str("_x");
        result.push_str(&format!("{high:04X}"));
        result.push('_');
    }

    Ok(result)
}
