use super::{methods, traits::ToPs};
use ironposh_psrp::PsValue;

impl ToPs for std::collections::HashMap<String, PsValue> {
    fn to_ps(_v: std::collections::HashMap<String, PsValue>) -> Option<PsValue> {
        todo!("Implement HashMap<String, PsValue> to PsValue conversion")
    }
}

impl ToPs for methods::PSCredential {
    fn to_ps(_v: methods::PSCredential) -> Option<PsValue> {
        todo!("Implement PSCredential to PsValue conversion")
    }
}

impl ToPs for Vec<i32> {
    fn to_ps(_v: Vec<i32>) -> Option<PsValue> {
        todo!("Implement Vec<i32> to PsValue conversion")
    }
}

impl ToPs for methods::KeyInfo {
    fn to_ps(_v: methods::KeyInfo) -> Option<PsValue> {
        todo!("Implement KeyInfo to PsValue conversion")
    }
}

impl ToPs for Vec<Vec<methods::BufferCell>> {
    fn to_ps(_v: Vec<Vec<methods::BufferCell>>) -> Option<PsValue> {
        todo!("Implement Vec<Vec<BufferCell>> to PsValue conversion")
    }
}

impl ToPs for methods::Coordinates {
    fn to_ps(_v: methods::Coordinates) -> Option<PsValue> {
        todo!("Implement Coordinates to PsValue conversion")
    }
}

impl ToPs for methods::Size {
    fn to_ps(_v: methods::Size) -> Option<PsValue> {
        todo!("Implement Size to PsValue conversion")
    }
}
