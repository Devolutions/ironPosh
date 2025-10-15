use std::collections::HashMap;

use super::{methods, traits::ToPs};
use ironposh_psrp::PsValue;

impl<S: ::std::hash::BuildHasher> ToPs for HashMap<String, PsValue, S> {
    fn to_ps(_v: Self) -> Option<PsValue> {
        todo!("Implement HashMap<String, PsValue> to PsValue conversion")
    }
}

impl ToPs for methods::PSCredential {
    fn to_ps(_v: Self) -> Option<PsValue> {
        todo!("Implement PSCredential to PsValue conversion")
    }
}

impl ToPs for Vec<i32> {
    fn to_ps(_v: Self) -> Option<PsValue> {
        todo!("Implement Vec<i32> to PsValue conversion")
    }
}

impl ToPs for methods::KeyInfo {
    fn to_ps(_v: Self) -> Option<PsValue> {
        todo!("Implement KeyInfo to PsValue conversion")
    }
}

impl ToPs for Vec<Vec<methods::BufferCell>> {
    fn to_ps(_v: Self) -> Option<PsValue> {
        todo!("Implement Vec<Vec<BufferCell>> to PsValue conversion")
    }
}

impl ToPs for methods::Coordinates {
    fn to_ps(_v: Self) -> Option<PsValue> {
        todo!("Implement Coordinates to PsValue conversion")
    }
}

impl ToPs for methods::Size {
    fn to_ps(_v: Self) -> Option<PsValue> {
        todo!("Implement Size to PsValue conversion")
    }
}
