use super::{HostError, methods, traits::FromParams};
use ironposh_psrp::{ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsValue};
use tracing::{debug, trace};

fn list_items(value: &PsValue) -> Option<&[PsValue]> {
    let PsValue::Object(obj) = value else {
        return None;
    };
    match &obj.content {
        ComplexObjectContent::Container(
            Container::List(items) | Container::Stack(items) | Container::Queue(items),
        ) => Some(items),
        _ => None,
    }
}

fn obj_prop_i32(obj: &ComplexObject, keys: &[&str]) -> Option<i32> {
    keys.iter().find_map(|k| {
        obj.extended_properties
            .get(*k)
            .and_then(|p| p.value.as_i32())
    })
}

fn obj_prop_bool(obj: &ComplexObject, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(
        |k| match obj.extended_properties.get(*k).map(|p| &p.value) {
            Some(PsValue::Primitive(PsPrimitiveValue::Bool(b))) => Some(*b),
            _ => None,
        },
    )
}

fn obj_prop_string(obj: &ComplexObject, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| {
        obj.extended_properties
            .get(*k)
            .and_then(|p| p.value.as_string())
    })
}

fn obj_prop_value(obj: &ComplexObject, keys: &[&str]) -> Option<PsValue> {
    keys.iter()
        .find_map(|k| obj.extended_properties.get(*k).map(|p| p.value.clone()))
}

// Complex parameter type implementations
impl FromParams for (i32, i32, String) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 3 {
            return Err(HostError::InvalidParameters);
        }
        let fg = a[0].as_i32().ok_or(HostError::InvalidParameters)?;
        let bg = a[1].as_i32().ok_or(HostError::InvalidParameters)?;
        let value = a[2].as_string().ok_or(HostError::InvalidParameters)?;
        Ok((fg, bg, value))
    }
}

impl FromParams for (i64, methods::ProgressRecord) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 2 {
            return Err(HostError::InvalidParameters);
        }
        let source_id = a[0].as_i64().ok_or(HostError::InvalidParameters)?;

        // Extract ProgressRecord from the ComplexObject
        match &a[1] {
            PsValue::Object(complex_obj) => {
                // Extract required fields
                let activity = complex_obj
                    .extended_properties
                    .get("Activity")
                    .and_then(|prop| match &prop.value {
                        PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let activity_id = complex_obj
                    .extended_properties
                    .get("ActivityId")
                    .and_then(|prop| match &prop.value {
                        PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                        _ => None,
                    })
                    .unwrap_or(0);

                let status_description = complex_obj
                    .extended_properties
                    .get("StatusDescription")
                    .and_then(|prop| match &prop.value {
                        PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                        PsValue::Primitive(PsPrimitiveValue::Nil) => Some(String::new()),
                        _ => None,
                    })
                    .unwrap_or_else(String::new);

                let current_operation = complex_obj
                    .extended_properties
                    .get("CurrentOperation")
                    .and_then(|prop| match &prop.value {
                        PsValue::Primitive(PsPrimitiveValue::Str(s)) => Some(s.clone()),
                        PsValue::Primitive(PsPrimitiveValue::Nil) => Some(String::new()),
                        _ => None,
                    })
                    .unwrap_or_else(String::new);

                let parent_activity_id = complex_obj
                    .extended_properties
                    .get("ParentActivityId")
                    .and_then(|prop| match &prop.value {
                        PsValue::Primitive(PsPrimitiveValue::I32(id)) => Some(*id),
                        _ => None,
                    })
                    .unwrap_or(-1);

                let percent_complete = complex_obj
                    .extended_properties
                    .get("PercentComplete")
                    .and_then(|prop| match &prop.value {
                        PsValue::Primitive(PsPrimitiveValue::I32(percent)) => Some(*percent),
                        _ => None,
                    })
                    .unwrap_or(-1);

                let seconds_remaining = complex_obj
                    .extended_properties
                    .get("SecondsRemaining")
                    .and_then(|prop| match &prop.value {
                        PsValue::Primitive(PsPrimitiveValue::I32(seconds)) => Some(*seconds),
                        _ => None,
                    })
                    .unwrap_or(-1);

                // Extract the record type from the nested Type object
                let record_type = complex_obj
                    .extended_properties
                    .get("Type")
                    .and_then(|prop| match &prop.value {
                        PsValue::Object(type_obj) => match &type_obj.content {
                            ComplexObjectContent::PsEnums(enums) => Some(enums.value),
                            _ => None,
                        },
                        PsValue::Primitive(_) => None,
                    })
                    .unwrap_or(0);

                let progress_record = methods::ProgressRecord {
                    activity,
                    status_description,
                    current_operation,
                    activity_id,
                    parent_activity_id,
                    percent_complete,
                    seconds_remaining,
                    record_type,
                };

                Ok((source_id, progress_record))
            }
            PsValue::Primitive(_) => Err(HostError::InvalidParameters),
        }
    }
}

impl FromParams for (String, String, Vec<methods::FieldDescription>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 3 {
            return Err(HostError::InvalidParameters);
        }
        let caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let message = a[1].as_string().ok_or(HostError::InvalidParameters)?;

        let items = list_items(&a[2]).ok_or_else(|| {
            debug!(param = ?a[2], "FieldDescription list is not a supported container");
            HostError::InvalidParameters
        })?;

        trace!(count = items.len(), "deserializing FieldDescription list");
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let PsValue::Object(obj) = item else {
                return Err(HostError::InvalidParameters);
            };

            let name =
                obj_prop_string(obj, &["name", "Name"]).ok_or(HostError::InvalidParameters)?;
            let label = obj_prop_string(obj, &["label", "Label"]).unwrap_or_default();
            let help_message =
                obj_prop_string(obj, &["helpMessage", "HelpMessage"]).unwrap_or_default();
            let is_mandatory = obj_prop_bool(obj, &["isMandatory", "IsMandatory"]).unwrap_or(false);
            let parameter_type = obj_prop_string(obj, &["parameterType", "ParameterType"])
                .or_else(|| obj_prop_string(obj, &["parameterTypeName", "ParameterTypeName"]))
                .unwrap_or_default();
            let default_value =
                obj_prop_value(obj, &["defaultValue", "DefaultValue"]).and_then(|v| match v {
                    PsValue::Primitive(PsPrimitiveValue::Nil) => None,
                    other => Some(other),
                });

            out.push(methods::FieldDescription {
                name,
                label,
                help_message,
                is_mandatory,
                parameter_type,
                default_value,
            });
        }

        Ok((caption, message, out))
    }
}

impl FromParams for (String, String, String, String) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 4 {
            return Err(HostError::InvalidParameters);
        }
        let caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let message = a[1].as_string().ok_or(HostError::InvalidParameters)?;
        let user_name = a[2].as_string().ok_or(HostError::InvalidParameters)?;
        let target_name = a[3].as_string().ok_or(HostError::InvalidParameters)?;
        Ok((caption, message, user_name, target_name))
    }
}

impl FromParams for (String, String, String, String, i32, i32) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 6 {
            return Err(HostError::InvalidParameters);
        }
        let caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let message = a[1].as_string().ok_or(HostError::InvalidParameters)?;
        let user_name = a[2].as_string().ok_or(HostError::InvalidParameters)?;
        let target_name = a[3].as_string().ok_or(HostError::InvalidParameters)?;
        let allowed_types = a[4].as_i32().ok_or(HostError::InvalidParameters)?;
        let options = a[5].as_i32().ok_or(HostError::InvalidParameters)?;
        Ok((
            caption,
            message,
            user_name,
            target_name,
            allowed_types,
            options,
        ))
    }
}

impl FromParams for (String, String, Vec<methods::ChoiceDescription>, i32) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 4 {
            return Err(HostError::InvalidParameters);
        }
        let caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let message = a[1].as_string().ok_or(HostError::InvalidParameters)?;
        let default_choice = a[3].as_i32().ok_or(HostError::InvalidParameters)?;

        let items = list_items(&a[2]).ok_or_else(|| {
            debug!(param = ?a[2], "ChoiceDescription list is not a supported container");
            HostError::InvalidParameters
        })?;

        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let PsValue::Object(obj) = item else {
                return Err(HostError::InvalidParameters);
            };
            let label = obj_prop_string(obj, &["label", "Label"]).unwrap_or_default();
            let help_message =
                obj_prop_string(obj, &["helpMessage", "HelpMessage"]).unwrap_or_default();
            out.push(methods::ChoiceDescription {
                label,
                help_message,
            });
        }

        Ok((caption, message, out, default_choice))
    }
}

impl FromParams for (String, String, Vec<methods::ChoiceDescription>, Vec<i32>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 4 {
            return Err(HostError::InvalidParameters);
        }
        let caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let message = a[1].as_string().ok_or(HostError::InvalidParameters)?;

        let choice_items = list_items(&a[2]).ok_or_else(|| {
            debug!(param = ?a[2], "ChoiceDescription list is not a supported container");
            HostError::InvalidParameters
        })?;

        let mut choices = Vec::with_capacity(choice_items.len());
        for item in choice_items {
            let PsValue::Object(obj) = item else {
                return Err(HostError::InvalidParameters);
            };
            let label = obj_prop_string(obj, &["label", "Label"]).unwrap_or_default();
            let help_message =
                obj_prop_string(obj, &["helpMessage", "HelpMessage"]).unwrap_or_default();
            choices.push(methods::ChoiceDescription {
                label,
                help_message,
            });
        }

        let default_items = list_items(&a[3]).ok_or_else(|| {
            debug!(param = ?a[3], "DefaultChoice list is not a supported container");
            HostError::InvalidParameters
        })?;
        let mut defaults = Vec::with_capacity(default_items.len());
        for v in default_items {
            let idx = v.as_i32().ok_or(HostError::InvalidParameters)?;
            defaults.push(idx);
        }

        Ok((caption, message, choices, defaults))
    }
}

impl FromParams for methods::Coordinates {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }

        match &a[0] {
            PsValue::Object(obj) => {
                let x = obj
                    .extended_properties
                    .get("x")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                let y = obj
                    .extended_properties
                    .get("y")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                Ok(Self { x, y })
            }
            PsValue::Primitive(_) => Err(HostError::InvalidParameters),
        }
    }
}

impl FromParams for (methods::Coordinates,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        let coord = methods::Coordinates::from_params(a)?;
        Ok((coord,))
    }
}

impl FromParams for methods::Size {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }

        match &a[0] {
            PsValue::Object(obj) => {
                let width =
                    obj_prop_i32(obj, &["width", "Width"]).ok_or(HostError::InvalidParameters)?;
                let height =
                    obj_prop_i32(obj, &["height", "Height"]).ok_or(HostError::InvalidParameters)?;
                Ok(Self { width, height })
            }
            PsValue::Primitive(_) => Err(HostError::InvalidParameters),
        }
    }
}

impl FromParams for (methods::Size,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }
        let s = methods::Size::from_params(a)?;
        Ok((s,))
    }
}

impl FromParams for methods::Rectangle {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }

        match &a[0] {
            PsValue::Object(obj) => {
                let left = obj
                    .extended_properties
                    .get("left")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                let top = obj
                    .extended_properties
                    .get("top")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                let right = obj
                    .extended_properties
                    .get("right")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                let bottom = obj
                    .extended_properties
                    .get("bottom")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                Ok(Self {
                    left,
                    top,
                    right,
                    bottom,
                })
            }
            PsValue::Primitive(_) => Err(HostError::InvalidParameters),
        }
    }
}

impl FromParams for (methods::Coordinates, Vec<Vec<methods::BufferCell>>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 2 {
            return Err(HostError::InvalidParameters);
        }

        let coords = methods::Coordinates::from_params(&a[0..1])?;
        let rows = list_items(&a[1]).ok_or_else(|| {
            debug!(param = ?a[1], "BufferCell 2D array is not a supported container");
            HostError::InvalidParameters
        })?;

        let mut out_rows = Vec::with_capacity(rows.len());
        for row in rows {
            let cells = list_items(row).ok_or_else(|| {
                debug!(param = ?row, "BufferCell row is not a supported container");
                HostError::InvalidParameters
            })?;

            let mut out_cells = Vec::with_capacity(cells.len());
            for cell in cells {
                let bc = methods::BufferCell::from_params(std::slice::from_ref(cell))?;
                out_cells.push(bc);
            }
            out_rows.push(out_cells);
        }

        Ok((coords, out_rows))
    }
}

impl FromParams for (methods::Rectangle, methods::BufferCell) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 2 {
            return Err(HostError::InvalidParameters);
        }
        let rectangle = methods::Rectangle::from_params(&a[0..1])?;
        let buffer_cell = methods::BufferCell::from_params(&a[1..2])?;
        Ok((rectangle, buffer_cell))
    }
}

impl FromParams for (methods::Rectangle,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }
        let r = methods::Rectangle::from_params(a)?;
        Ok((r,))
    }
}

impl FromParams
    for (
        methods::Rectangle,
        methods::Coordinates,
        methods::Rectangle,
        methods::BufferCell,
    )
{
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 4 {
            return Err(HostError::InvalidParameters);
        }
        let source = methods::Rectangle::from_params(&a[0..1])?;
        let destination = methods::Coordinates::from_params(&a[1..2])?;
        let clip = methods::Rectangle::from_params(&a[2..3])?;
        let fill = methods::BufferCell::from_params(&a[3..4])?;
        Ok((source, destination, clip, fill))
    }
}

// BufferCell deserialization
impl FromParams for methods::BufferCell {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }

        match &a[0] {
            PsValue::Object(obj) => {
                let character = obj
                    .extended_properties
                    .get("character")
                    .and_then(|prop| {
                        if let PsValue::Primitive(ironposh_psrp::PsPrimitiveValue::Char(c)) =
                            &prop.value
                        {
                            Some(*c)
                        } else {
                            None
                        }
                    })
                    .ok_or(HostError::InvalidParameters)?;

                let foreground = obj
                    .extended_properties
                    .get("foregroundColor")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                let background = obj
                    .extended_properties
                    .get("backgroundColor")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                let flags = obj
                    .extended_properties
                    .get("bufferCellType")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;

                Ok(Self {
                    character,
                    foreground,
                    background,
                    flags,
                })
            }
            PsValue::Primitive(_) => Err(HostError::InvalidParameters),
        }
    }
}

impl FromParams for (PsValue,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }
        Ok((a[0].clone(),))
    }
}
