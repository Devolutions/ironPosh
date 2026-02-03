use crate::ps_value::{Container, PsPrimitiveValue, PsValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCompletion {
    pub current_match_index: i32,
    pub replacement_index: i32,
    pub replacement_length: i32,
    pub completion_matches: Vec<CompletionResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResult {
    pub completion_text: String,
    pub list_item_text: String,
    pub result_type: String,
    pub tool_tip: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandCompletionError {
    #[error("expected a PowerShell object for {context}, got {found}")]
    ExpectedObject {
        context: &'static str,
        found: &'static str,
    },

    #[error("missing property {name} in {context}")]
    MissingProperty {
        context: &'static str,
        name: &'static str,
    },

    #[error("unexpected type for {context}.{name}: expected {expected}, got {found}")]
    UnexpectedType {
        context: &'static str,
        name: &'static str,
        expected: &'static str,
        found: &'static str,
    },
}

impl TryFrom<&PsValue> for CommandCompletion {
    type Error = CommandCompletionError;

    fn try_from(value: &PsValue) -> Result<Self, Self::Error> {
        let obj = value
            .as_object()
            .ok_or_else(|| CommandCompletionError::ExpectedObject {
                context: "CommandCompletion",
                found: ps_value_kind(value),
            })?;

        let current_match_index = get_i32(value, "CommandCompletion", "CurrentMatchIndex")?;
        let replacement_index = get_i32(value, "CommandCompletion", "ReplacementIndex")?;
        let replacement_length = get_i32(value, "CommandCompletion", "ReplacementLength")?;

        let matches_value = obj.adapted_properties.get("CompletionMatches").ok_or(
            CommandCompletionError::MissingProperty {
                context: "CommandCompletion",
                name: "CompletionMatches",
            },
        )?;

        let matches_obj = matches_value.value.as_object().ok_or_else(|| {
            CommandCompletionError::ExpectedObject {
                context: "CommandCompletion.CompletionMatches",
                found: ps_value_kind(&matches_value.value),
            }
        })?;

        let Container::List(items) = matches_obj.content.container().ok_or_else(|| {
            CommandCompletionError::UnexpectedType {
                context: "CommandCompletion",
                name: "CompletionMatches",
                expected: "Container(List)",
                found: complex_content_kind(&matches_obj.content),
            }
        })?
        else {
            return Err(CommandCompletionError::UnexpectedType {
                context: "CommandCompletion",
                name: "CompletionMatches",
                expected: "Container(List)",
                found: "Container(non-list)",
            });
        };

        let completion_matches = items
            .iter()
            .map(CompletionResult::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            current_match_index,
            replacement_index,
            replacement_length,
            completion_matches,
        })
    }
}

impl TryFrom<&PsValue> for CompletionResult {
    type Error = CommandCompletionError;

    fn try_from(value: &PsValue) -> Result<Self, Self::Error> {
        let obj = value
            .as_object()
            .ok_or_else(|| CommandCompletionError::ExpectedObject {
                context: "CompletionResult",
                found: ps_value_kind(value),
            })?;

        let completion_text = get_string_from_obj(obj, "CompletionResult", "CompletionText")?;
        let list_item_text = get_string_from_obj(obj, "CompletionResult", "ListItemText")?;
        let result_type = get_string_from_obj(obj, "CompletionResult", "ResultType")?;
        let tool_tip = get_string_from_obj(obj, "CompletionResult", "ToolTip")?;

        Ok(Self {
            completion_text,
            list_item_text,
            result_type,
            tool_tip,
        })
    }
}

fn get_i32(
    value: &PsValue,
    context: &'static str,
    name: &'static str,
) -> Result<i32, CommandCompletionError> {
    let obj = value
        .as_object()
        .ok_or_else(|| CommandCompletionError::ExpectedObject {
            context,
            found: ps_value_kind(value),
        })?;
    let prop = obj
        .adapted_properties
        .get(name)
        .ok_or(CommandCompletionError::MissingProperty { context, name })?;
    match &prop.value {
        PsValue::Primitive(PsPrimitiveValue::I32(v)) => Ok(*v),
        other => Err(CommandCompletionError::UnexpectedType {
            context,
            name,
            expected: "I32",
            found: ps_value_kind(other),
        }),
    }
}

fn get_string_from_obj(
    obj: &crate::ps_value::ComplexObject,
    context: &'static str,
    name: &'static str,
) -> Result<String, CommandCompletionError> {
    let prop = obj
        .adapted_properties
        .get(name)
        .ok_or(CommandCompletionError::MissingProperty { context, name })?;
    match &prop.value {
        PsValue::Primitive(PsPrimitiveValue::Str(v)) => Ok(v.clone()),
        other => Err(CommandCompletionError::UnexpectedType {
            context,
            name,
            expected: "String",
            found: ps_value_kind(other),
        }),
    }
}

fn ps_value_kind(v: &PsValue) -> &'static str {
    match v {
        PsValue::Primitive(_) => "Primitive",
        PsValue::Object(_) => "Object",
    }
}

fn complex_content_kind(content: &crate::ps_value::ComplexObjectContent) -> &'static str {
    match content {
        crate::ps_value::ComplexObjectContent::Standard => "Standard",
        crate::ps_value::ComplexObjectContent::ExtendedPrimitive(_) => "ExtendedPrimitive",
        crate::ps_value::ComplexObjectContent::Container(_) => "Container",
        crate::ps_value::ComplexObjectContent::PsEnums(_) => "PsEnums",
    }
}

trait ComplexObjectContentExt {
    fn container(&self) -> Option<&Container>;
}

impl ComplexObjectContentExt for crate::ps_value::ComplexObjectContent {
    fn container(&self) -> Option<&Container> {
        match self {
            Self::Container(c) => Some(c),
            Self::Standard | Self::ExtendedPrimitive(_) | Self::PsEnums(_) => None,
        }
    }
}
