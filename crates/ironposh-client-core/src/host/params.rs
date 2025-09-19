use super::{HostError, methods, traits::FromParams};
use ironposh_psrp::PsValue;

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
        let _source_id = a[0].as_i64().ok_or(HostError::InvalidParameters)?;
        // ProgressRecord deserialization needs proper implementation
        todo!("Implement ProgressRecord deserialization from PsValue")
    }
}

impl FromParams for (String, String, Vec<methods::FieldDescription>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 3 {
            return Err(HostError::InvalidParameters);
        }
        let _caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let _message = a[1].as_string().ok_or(HostError::InvalidParameters)?;
        // FieldDescription vector deserialization needs proper implementation
        todo!("Implement Vec<FieldDescription> deserialization from PsValue")
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
        let _caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let _message = a[1].as_string().ok_or(HostError::InvalidParameters)?;
        // ChoiceDescription vector deserialization needs proper implementation
        let _default_choice = a[3].as_i32().ok_or(HostError::InvalidParameters)?;
        todo!("Implement Vec<ChoiceDescription> deserialization from PsValue")
    }
}

impl FromParams for (String, String, Vec<methods::ChoiceDescription>, Vec<i32>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 4 {
            return Err(HostError::InvalidParameters);
        }
        let _caption = a[0].as_string().ok_or(HostError::InvalidParameters)?;
        let _message = a[1].as_string().ok_or(HostError::InvalidParameters)?;
        // Complex deserialization needs proper implementation
        todo!("Implement Vec<ChoiceDescription> and Vec<i32> deserialization from PsValue")
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

                Ok(methods::Coordinates { x, y })
            }
            _ => Err(HostError::InvalidParameters),
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
    fn from_params(_a: &[PsValue]) -> Result<Self, HostError> {
        todo!("Implement Size deserialization from PsValue")
    }
}

impl FromParams for (methods::Size,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }
        todo!("Implement Size deserialization from PsValue")
    }
}

impl FromParams for methods::Rectangle {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 1 {
            return Err(HostError::InvalidParameters);
        }
        
        match &a[0] {
            PsValue::Object(obj) => {
                let left = obj.extended_properties
                    .get("left")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                
                let top = obj.extended_properties
                    .get("top")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                
                let right = obj.extended_properties
                    .get("right")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                
                let bottom = obj.extended_properties
                    .get("bottom")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                
                Ok(methods::Rectangle { left, top, right, bottom })
            }
            _ => Err(HostError::InvalidParameters),
        }
    }
}

impl FromParams for (methods::Coordinates, Vec<Vec<methods::BufferCell>>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.len() != 2 {
            return Err(HostError::InvalidParameters);
        }
        todo!("Implement complex BufferCell array deserialization from PsValue")
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
        todo!("Implement Rectangle deserialization from PsValue")
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
        todo!("Implement complex ScrollBufferContents parameter deserialization from PsValue")
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
                let character = obj.extended_properties
                    .get("character")
                    .and_then(|prop| {
                        if let PsValue::Primitive(ironposh_psrp::PsPrimitiveValue::Char(c)) = &prop.value {
                            Some(*c)
                        } else {
                            None
                        }
                    })
                    .ok_or(HostError::InvalidParameters)?;
                
                let foreground = obj.extended_properties
                    .get("foregroundColor")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                
                let background = obj.extended_properties
                    .get("backgroundColor")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                
                let flags = obj.extended_properties
                    .get("bufferCellType")
                    .and_then(|prop| prop.value.as_i32())
                    .ok_or(HostError::InvalidParameters)?;
                
                Ok(methods::BufferCell { character, foreground, background, flags })
            }
            _ => Err(HostError::InvalidParameters),
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
