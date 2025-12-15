use std::{borrow::Cow, collections::BTreeMap, fmt::Write};

use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
};

use tracing::{debug, error};

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct ErrorRecord {
    /// The error message
    pub message: String,
    /// The command name that caused the error
    #[builder(default)]
    pub command_name: Option<String>,
    /// Whether this was thrown from a throw statement
    #[builder(default = false)]
    pub was_thrown_from_throw_statement: bool,
    /// The fully qualified error ID
    #[builder(default)]
    pub fully_qualified_error_id: Option<String>,
    /// The target object that caused the error
    #[builder(default)]
    pub target_object: Option<String>,
    /// The exception that caused this error
    #[builder(default)]
    pub exception: Option<PsValue>,
    /// Error category information
    #[builder(default)]
    pub error_category: Option<ErrorCategory>,
    /// Whether to serialize extended information
    #[builder(default = false)]
    pub serialize_extended_info: bool,
    /// Invocation information (if available)
    #[builder(default)]
    pub invocation_info: Option<PsValue>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions {
    /// Include the category summary line ("ObjectNotFound: ...").
    pub include_category: bool,
    /// Include position info (file:line:col + caret block) when available.
    pub include_position: bool,
    /// If true, trim trailing newlines from each section.
    pub trim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct ErrorCategory {
    /// The error category number
    pub category: i32,
    /// The activity that caused the error
    #[builder(default)]
    pub activity: Option<String>,
    /// The reason for the error
    #[builder(default)]
    pub reason: Option<String>,
    /// The target name
    #[builder(default)]
    pub target_name: Option<String>,
    /// The target type
    #[builder(default)]
    pub target_type: Option<String>,
    /// The error category message
    #[builder(default)]
    pub message: Option<String>,
}

impl ErrorRecord {
    /// PS 7 "ConciseView": just the main message.
    pub fn render_concise(&self) -> String {
        normalize(self.message.as_str())
    }

    /// Classic "NormalView": message + category + position (when available).
    pub fn render_normal(&self) -> String {
        self.render_with_options(RenderOptions {
            include_category: true,
            include_position: true,
            trim: true,
        })
    }

    /// Full control over what to include.
    pub fn render_with_options(&self, opts: RenderOptions) -> String {
        let mut out = String::new();

        // 1) Primary message
        push_line(&mut out, &normalize(&self.message), opts.trim);

        // 2) Category line (short diagnostic summary)
        if opts.include_category
            && let Some(cat) = self
                .error_category
                .as_ref()
                .and_then(|c| c.message.as_ref())
                .map(|s| normalize(s))
            && !cat.is_empty()
        {
            push_line(&mut out, &cat, opts.trim);
        }

        // 3) Position block (from InvocationInfo if present)
        if opts.include_position
            && let Some(pos) = extract_position_block(self.invocation_info.as_ref())
        {
            push_line(&mut out, &pos, opts.trim);
        }

        out
    }
}

impl PsObjectWithType for ErrorRecord {
    fn message_type(&self) -> MessageType {
        MessageType::ErrorRecord
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

impl From<ErrorRecord> for ComplexObject {
    #[expect(clippy::too_many_lines)]
    fn from(record: ErrorRecord) -> Self {
        let mut extended_properties = BTreeMap::new();

        // Core error record properties
        extended_properties.insert(
            "ErrorRecord".to_string(),
            PsProperty {
                name: "ErrorRecord".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(record.message.clone())),
            },
        );

        if let Some(command_name) = record.command_name {
            extended_properties.insert(
                "CommandName".to_string(),
                PsProperty {
                    name: "CommandName".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(command_name)),
                },
            );
        }

        extended_properties.insert(
            "WasThrownFromThrowStatement".to_string(),
            PsProperty {
                name: "WasThrownFromThrowStatement".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(
                    record.was_thrown_from_throw_statement,
                )),
            },
        );

        extended_properties.insert(
            "Message".to_string(),
            PsProperty {
                name: "Message".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Str(record.message.clone())),
            },
        );

        if let Some(exception) = record.exception {
            extended_properties.insert(
                "Exception".to_string(),
                PsProperty {
                    name: "Exception".to_string(),
                    value: exception,
                },
            );
        }

        if let Some(target_object) = record.target_object {
            extended_properties.insert(
                "TargetObject".to_string(),
                PsProperty {
                    name: "TargetObject".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(target_object)),
                },
            );
        }

        if let Some(fully_qualified_error_id) = record.fully_qualified_error_id {
            extended_properties.insert(
                "FullyQualifiedErrorId".to_string(),
                PsProperty {
                    name: "FullyQualifiedErrorId".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Str(fully_qualified_error_id)),
                },
            );
        }

        if let Some(invocation_info) = record.invocation_info {
            extended_properties.insert(
                "InvocationInfo".to_string(),
                PsProperty {
                    name: "InvocationInfo".to_string(),
                    value: invocation_info,
                },
            );
        }

        // Error category properties
        if let Some(error_category) = record.error_category {
            extended_properties.insert(
                "ErrorCategory_Category".to_string(),
                PsProperty {
                    name: "ErrorCategory_Category".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::I32(error_category.category)),
                },
            );

            if let Some(activity) = error_category.activity {
                extended_properties.insert(
                    "ErrorCategory_Activity".to_string(),
                    PsProperty {
                        name: "ErrorCategory_Activity".to_string(),
                        value: PsValue::Primitive(PsPrimitiveValue::Str(activity)),
                    },
                );
            }

            if let Some(reason) = error_category.reason {
                extended_properties.insert(
                    "ErrorCategory_Reason".to_string(),
                    PsProperty {
                        name: "ErrorCategory_Reason".to_string(),
                        value: PsValue::Primitive(PsPrimitiveValue::Str(reason)),
                    },
                );
            }

            if let Some(target_name) = error_category.target_name {
                extended_properties.insert(
                    "ErrorCategory_TargetName".to_string(),
                    PsProperty {
                        name: "ErrorCategory_TargetName".to_string(),
                        value: PsValue::Primitive(PsPrimitiveValue::Str(target_name)),
                    },
                );
            }

            if let Some(target_type) = error_category.target_type {
                extended_properties.insert(
                    "ErrorCategory_TargetType".to_string(),
                    PsProperty {
                        name: "ErrorCategory_TargetType".to_string(),
                        value: PsValue::Primitive(PsPrimitiveValue::Str(target_type)),
                    },
                );
            }

            if let Some(message) = error_category.message {
                extended_properties.insert(
                    "ErrorCategory_Message".to_string(),
                    PsProperty {
                        name: "ErrorCategory_Message".to_string(),
                        value: PsValue::Primitive(PsPrimitiveValue::Str(message)),
                    },
                );
            }
        }

        extended_properties.insert(
            "SerializeExtendedInfo".to_string(),
            PsProperty {
                name: "SerializeExtendedInfo".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Bool(record.serialize_extended_info)),
            },
        );

        Self {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Management.Automation.ErrorRecord"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: Some(record.message),
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<PsValue> for ErrorRecord {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: PsValue) -> Result<Self, Self::Error> {
        match value {
            PsValue::Object(obj) => Self::try_from(obj),
            PsValue::Primitive(_) => Err(Self::Error::InvalidMessage(
                "Expected ComplexObject for ErrorRecord".to_string(),
            )),
        }
    }
}

impl TryFrom<ComplexObject> for ErrorRecord {
    type Error = crate::PowerShellRemotingError;

    #[expect(clippy::too_many_lines)]
    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        // Debug logging to understand what properties are actually available
        debug!(?value.extended_properties, "ErrorRecord extended_properties");

        // Try multiple locations for the message:
        // 1. Top-level "Message" property
        // 2. Top-level "ErrorRecord" property
        // 3. Extract from nested Exception object
        // 4. Use the ToString value as fallback
        let message = value
            .extended_properties
            .get("Message")
            .or_else(|| value.extended_properties.get("ErrorRecord"))
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            })
            .or_else(|| {
                // Try to extract message from Exception object's properties
                value.extended_properties
                    .get("Exception")
                    .and_then(|exception_prop| match &exception_prop.value {
                        PsValue::Object(exception_obj) => {
                            exception_obj.extended_properties
                                .get("Message")
                                .or_else(|| exception_obj.extended_properties.get("ErrorRecord"))
                                .and_then(|prop| match &prop.value {
                                    PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                                    _ => None,
                                })
                        }
                        PsValue::Primitive(_) => None,
                    })
            })
            .or_else(|| {
                // Fallback to the ComplexObject's toString value
                value.to_string.clone()
            })
            .ok_or_else(|| {
                // Enhanced error message with available property names for debugging
                let available_properties: Vec<&String> = value.extended_properties.keys().collect();
                error!(?available_properties, "ErrorRecord TryFrom failed - available properties");
                Self::Error::InvalidMessage(
                    format!("Missing Message or ErrorRecord property in all expected locations. Available properties: {available_properties:?}")
                )
            })?;

        debug!(?message, "ErrorRecord message found");

        let command_name = value
            .extended_properties
            .get("CommandName")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            })
            .or_else(|| {
                // Try to extract CommandName from Exception object's properties
                value
                    .extended_properties
                    .get("Exception")
                    .and_then(|exception_prop| match &exception_prop.value {
                        PsValue::Object(exception_obj) => exception_obj
                            .extended_properties
                            .get("CommandName")
                            .and_then(|prop| match &prop.value {
                                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                                _ => None,
                            }),
                        PsValue::Primitive(_) => None,
                    })
            });

        let was_thrown_from_throw_statement = value
            .extended_properties
            .get("WasThrownFromThrowStatement")
            .is_some_and(|prop| {
                if let PsValue::Primitive(PsPrimitiveValue::Bool(b)) = prop.value {
                    b
                } else {
                    false
                }
            });

        let fully_qualified_error_id = value
            .extended_properties
            .get("FullyQualifiedErrorId")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let target_object = value
            .extended_properties
            .get("TargetObject")
            .and_then(|prop| match &prop.value {
                PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                _ => None,
            });

        let exception = value
            .extended_properties
            .get("Exception")
            .map(|prop| prop.value.clone());

        let invocation_info = value
            .extended_properties
            .get("InvocationInfo")
            .map(|prop| prop.value.clone())
            .filter(|v| !matches!(v, PsValue::Primitive(PsPrimitiveValue::Nil)));

        let serialize_extended_info = value
            .extended_properties
            .get("SerializeExtendedInfo")
            .is_some_and(|prop| {
                if let PsValue::Primitive(PsPrimitiveValue::Bool(b)) = prop.value {
                    b
                } else {
                    false
                }
            });

        // Parse error category
        let error_category =
            if let Some(category_prop) = value.extended_properties.get("ErrorCategory_Category") {
                if let PsValue::Primitive(PsPrimitiveValue::I32(category)) = &category_prop.value {
                    let activity = value
                        .extended_properties
                        .get("ErrorCategory_Activity")
                        .and_then(|prop| match &prop.value {
                            PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                            _ => None,
                        });

                    let reason = value
                        .extended_properties
                        .get("ErrorCategory_Reason")
                        .and_then(|prop| match &prop.value {
                            PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                            _ => None,
                        });

                    let target_name = value
                        .extended_properties
                        .get("ErrorCategory_TargetName")
                        .and_then(|prop| match &prop.value {
                            PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                            _ => None,
                        });

                    let target_type = value
                        .extended_properties
                        .get("ErrorCategory_TargetType")
                        .and_then(|prop| match &prop.value {
                            PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                            _ => None,
                        });

                    let category_message = value
                        .extended_properties
                        .get("ErrorCategory_Message")
                        .and_then(|prop| match &prop.value {
                            PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                            _ => None,
                        });

                    Some(
                        ErrorCategory::builder()
                            .category(*category)
                            .activity(activity)
                            .reason(reason)
                            .target_name(target_name)
                            .target_type(target_type)
                            .message(category_message)
                            .build(),
                    )
                } else {
                    None
                }
            } else {
                None
            };

        Ok(Self::builder()
            .message(message)
            .command_name(command_name)
            .was_thrown_from_throw_statement(was_thrown_from_throw_statement)
            .fully_qualified_error_id(fully_qualified_error_id)
            .target_object(target_object)
            .exception(exception)
            .error_category(error_category)
            .serialize_extended_info(serialize_extended_info)
            .invocation_info(invocation_info)
            .build())
    }
}

/* ---------------------- helpers ---------------------- */

fn normalize(s: &str) -> String {
    // PSRP sometimes embeds CRLF as "_x000D__x000A_"
    s.replace("_x000D__x000A_", "\r\n")
}

fn push_line(buf: &mut String, s: &str, trim: bool) {
    if s.is_empty() {
        return;
    }
    if trim {
        let s = s.trim_end_matches(['\r', '\n']);
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(s);
    } else {
        if !buf.is_empty() && !buf.ends_with('\n') {
            buf.push('\n');
        }
        buf.push_str(s);
        if !s.ends_with('\n') {
            buf.push('\n');
        }
    }
}

/// Extract a ready-to-print "at path:line char:col\n+ code\n+  ~~" block
/// from InvocationInfo when available. Falls back gracefully.
fn extract_position_block(invocation_info: Option<&PsValue>) -> Option<String> {
    let Some(PsValue::Object(obj)) = invocation_info else {
        return None;
    };

    // 1) If PowerShell already provided PositionMessage, use it.
    if let Some(pm) = get_str(&obj.extended_properties, "PositionMessage") {
        let pm = normalize(&pm);
        if !pm.is_empty() {
            return Some(pm);
        }
    }

    // 2) Otherwise synthesize from ScriptName/ScriptLineNumber/OffsetInLine/Line/LineText
    let script = get_str(&obj.extended_properties, "ScriptName")
        .or_else(|| get_str(&obj.extended_properties, "ScriptPath"))
        .unwrap_or_default();

    let line = get_i32(&obj.extended_properties, "ScriptLineNumber").unwrap_or(0);
    let col = get_i32(&obj.extended_properties, "OffsetInLine").unwrap_or(0);

    // Some serializations include the line text
    let line_text = get_str(&obj.extended_properties, "Line")
        .or_else(|| get_str(&obj.extended_properties, "LineText"))
        .unwrap_or_default();

    if script.is_empty() && line == 0 && col == 0 && line_text.is_empty() {
        return None;
    }

    let mut block = String::new();
    if !script.is_empty() || line > 0 || col > 0 {
        write!(
            block,
            "at {}{}{}",
            if script.is_empty() {
                "<unknown>"
            } else {
                &script
            },
            if line > 0 {
                format!(":{line}")
            } else {
                String::new()
            },
            if col > 0 {
                format!(" char:{col}")
            } else {
                String::new()
            },
        )
        .unwrap();
    }

    if !line_text.is_empty() {
        // PowerShell prints a two-line code excerpt with a caret/tilde underline
        let underline = if col > 0 {
            // PS caret typically starts at column; we'll use ~ to mark one token column.
            let mut u = String::new();
            // Avoid panics on large col by capping
            let pad = col.saturating_sub(1) as usize;
            u.push_str(&" ".repeat(pad));
            u.push_str("~~");
            u
        } else {
            String::from("~")
        };

        if !block.is_empty() {
            block.push('\n');
        }
        block.push_str("+ ");
        block.push_str(&line_text);
        if !block.ends_with('\n') {
            block.push('\n');
        }
        block.push_str("+ ");
        block.push_str(&underline);
    }

    if block.is_empty() { None } else { Some(block) }
}

/* ------- tiny PsValue extractors for InvocationInfo ------- */

fn get_str(map: &BTreeMap<String, PsProperty>, key: &str) -> Option<String> {
    map.get(key).and_then(|p| match &p.value {
        PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
        // Some shapes might stick a char
        PsValue::Primitive(PsPrimitiveValue::Char(c)) => Some(c.to_string()),
        // Or nest the value as an object ToString()
        PsValue::Object(o) => o.to_string.clone(),
        PsValue::Primitive(_) => None,
    })
}

fn get_i32(map: &BTreeMap<String, PsProperty>, key: &str) -> Option<i32> {
    map.get(key).and_then(|p| match &p.value {
        PsValue::Primitive(PsPrimitiveValue::I32(v)) => Some(*v),
        PsValue::Primitive(PsPrimitiveValue::I64(v)) => i32::try_from(*v).ok(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_record_basic() {
        let record = ErrorRecord::builder()
            .message("Test error message".to_string())
            .command_name(Some("Test-Command".to_string()))
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = ErrorRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_error_record_with_category() {
        let category = ErrorCategory::builder()
            .category(13)
            .reason(Some("CommandNotFoundException".to_string()))
            .target_name(Some("ea".to_string()))
            .target_type(Some("String".to_string()))
            .message(Some(
                "ObjectNotFound: (ea:String) [], CommandNotFoundException".to_string(),
            ))
            .build();

        let record = ErrorRecord::builder()
            .message("The term 'ea' is not recognized".to_string())
            .command_name(Some("ea".to_string()))
            .fully_qualified_error_id(Some("CommandNotFoundException".to_string()))
            .target_object(Some("ea".to_string()))
            .error_category(Some(category))
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = ErrorRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_message_type() {
        let record = ErrorRecord::builder().message("Test".to_string()).build();

        assert_eq!(record.message_type().value(), 0x00041005);
    }

    #[test]
    fn test_to_string_property() {
        let record = ErrorRecord::builder()
            .message("Test error message".to_string())
            .build();

        let complex_obj = ComplexObject::from(record);
        assert_eq!(
            complex_obj.to_string,
            Some("Test error message".to_string())
        );
    }

    #[test]
    fn test_type_names() {
        let record = ErrorRecord::builder().message("Test".to_string()).build();

        let complex_obj = ComplexObject::from(record);
        let type_def = complex_obj.type_def.unwrap();
        assert_eq!(type_def.type_names.len(), 2);
        assert_eq!(
            type_def.type_names[0].as_ref(),
            "System.Management.Automation.ErrorRecord"
        );
        assert_eq!(type_def.type_names[1].as_ref(), "System.Object");
    }

    #[test]
    fn test_command_not_found_error() {
        let category = ErrorCategory::builder()
            .category(13)
            .activity(Some(String::new()))
            .reason(Some("CommandNotFoundException".to_string()))
            .target_name(Some("ea".to_string()))
            .target_type(Some("String".to_string()))
            .message(Some(
                "ObjectNotFound: (ea:String) [], CommandNotFoundException".to_string(),
            ))
            .build();

        let record = ErrorRecord::builder()
            .message("The term 'ea' is not recognized as the name of a cmdlet, function, script file, or operable program. Check the spelling of the name, or if a path was included, verify that the path is correct and try again.".to_string())
            .command_name(Some("ea".to_string()))
            .was_thrown_from_throw_statement(false)
            .fully_qualified_error_id(Some("CommandNotFoundException".to_string()))
            .target_object(Some("ea".to_string()))
            .error_category(Some(category))
            .serialize_extended_info(false)
            .build();

        let complex_obj = ComplexObject::from(record.clone());
        let roundtrip = ErrorRecord::try_from(complex_obj).unwrap();

        assert_eq!(record, roundtrip);
    }

    #[test]
    fn test_render_concise() {
        let record = ErrorRecord::builder()
            .message("The term 'ea' is not recognized_x000D__x000A_Try again.".to_string())
            .build();

        let rendered = record.render_concise();
        assert_eq!(rendered, "The term 'ea' is not recognized\r\nTry again.");
    }

    #[test]
    fn test_render_normal() {
        let category = ErrorCategory::builder()
            .category(13)
            .message(Some(
                "ObjectNotFound: (ea:String) [], CommandNotFoundException".to_string(),
            ))
            .build();

        let record = ErrorRecord::builder()
            .message("The term 'ea' is not recognized".to_string())
            .error_category(Some(category))
            .build();

        let rendered = record.render_normal();
        assert!(rendered.contains("The term 'ea' is not recognized"));
        assert!(rendered.contains("ObjectNotFound: (ea:String) [], CommandNotFoundException"));
    }

    #[test]
    fn test_render_with_options() {
        let category = ErrorCategory::builder()
            .category(13)
            .message(Some(
                "ObjectNotFound: (ea:String) [], CommandNotFoundException".to_string(),
            ))
            .build();

        let record = ErrorRecord::builder()
            .message("Test error".to_string())
            .error_category(Some(category))
            .build();

        // Test with category only
        let rendered = record.render_with_options(RenderOptions {
            include_category: true,
            include_position: false,
            trim: true,
        });
        assert!(rendered.contains("Test error"));
        assert!(rendered.contains("ObjectNotFound"));

        // Test with no category
        let rendered = record.render_with_options(RenderOptions {
            include_category: false,
            include_position: false,
            trim: true,
        });
        assert_eq!(rendered, "Test error");
    }
}
