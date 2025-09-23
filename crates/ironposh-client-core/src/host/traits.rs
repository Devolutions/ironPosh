use super::HostError;
use ironposh_psrp::PsValue;

/// Sealed trait for compile-time method type safety
pub trait Method: sealed::Sealed {
    const ID: i32;
    const NAME: &'static str;
    type Params;
    type Return;

    fn should_send_response() -> bool;
}

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Parameter extraction from pipeline values
pub trait FromParams: Sized {
    fn from_params(args: &[PsValue]) -> Result<Self, HostError>;
}

/// Return value encoding to pipeline values
pub trait ToPs {
    fn to_ps(v: Self) -> Option<PsValue>;
}

// Implement basic parameter/return conversions
impl FromParams for () {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        if a.is_empty() {
            Ok(())
        } else {
            Err(HostError::InvalidParameters)
        }
    }
}

impl FromParams for String {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        a.get(0)
            .and_then(|v| v.as_string())
            .ok_or(HostError::InvalidParameters)
    }
}

impl FromParams for i32 {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        a.get(0)
            .and_then(|v| v.as_i32())
            .ok_or(HostError::InvalidParameters)
    }
}

impl FromParams for i64 {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        a.get(0)
            .and_then(|v| v.as_i64())
            .ok_or(HostError::InvalidParameters)
    }
}

impl FromParams for (i32,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        let param = a
            .get(0)
            .and_then(|v| v.as_i32())
            .ok_or(HostError::InvalidParameters)?;
        Ok((param,))
    }
}

impl FromParams for (String,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        let param = a
            .get(0)
            .and_then(|v| v.as_string())
            .ok_or(HostError::InvalidParameters)?;
        Ok((param,))
    }
}

// Basic return type conversions
impl ToPs for String {
    fn to_ps(v: String) -> Option<PsValue> {
        Some(PsValue::from(v))
    }
}

impl ToPs for i32 {
    fn to_ps(v: i32) -> Option<PsValue> {
        Some(PsValue::from(v))
    }
}

impl ToPs for bool {
    fn to_ps(v: bool) -> Option<PsValue> {
        Some(PsValue::from(v))
    }
}

impl ToPs for uuid::Uuid {
    fn to_ps(v: uuid::Uuid) -> Option<PsValue> {
        Some(PsValue::from(v.to_string()))
    }
}

impl ToPs for Vec<u8> {
    fn to_ps(v: Vec<u8>) -> Option<PsValue> {
        Some(PsValue::from(v))
    }
}

impl ToPs for PsValue {
    fn to_ps(v: PsValue) -> Option<PsValue> {
        Some(v)
    }
}

impl ToPs for () {
    fn to_ps(_: ()) -> Option<PsValue> {
        None // Void methods don't return values
    }
}
