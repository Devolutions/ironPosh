use super::{HostError, methods, traits::FromParams};
use ironposh_psrp::PsValue;
use ironposh_psrp::ps_value::FromPsValue;

// Host-call parameter extraction. The CLIXML→type conversion is fully
// macro-derived (`FromPsValue`); `FromParams` here is only the positional
// adapter the host dispatch calls — it pulls argument `i` and delegates.
fn arg<T: FromPsValue>(a: &[PsValue], i: usize) -> Result<T, HostError> {
    T::from_ps_value(a.get(i).ok_or(HostError::InvalidParameters)?)
        .map_err(|_| HostError::InvalidParameters)
}

impl FromParams for (i32, i32, String) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?, arg(a, 2)?))
    }
}

impl FromParams for (i64, methods::ProgressRecord) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?))
    }
}

impl FromParams for (String, String, Vec<methods::FieldDescription>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?, arg(a, 2)?))
    }
}

impl FromParams for (String, String, String, String) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?, arg(a, 2)?, arg(a, 3)?))
    }
}

impl FromParams for (String, String, String, String, i32, i32) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((
            arg(a, 0)?,
            arg(a, 1)?,
            arg(a, 2)?,
            arg(a, 3)?,
            arg(a, 4)?,
            arg(a, 5)?,
        ))
    }
}

impl FromParams for (String, String, Vec<methods::ChoiceDescription>, i32) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?, arg(a, 2)?, arg(a, 3)?))
    }
}

impl FromParams for (String, String, Vec<methods::ChoiceDescription>, Vec<i32>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?, arg(a, 2)?, arg(a, 3)?))
    }
}

impl FromParams for (methods::Coordinates,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?,))
    }
}

impl FromParams for (methods::Size,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?,))
    }
}

impl FromParams for (methods::Coordinates, Vec<Vec<methods::BufferCell>>) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?))
    }
}

impl FromParams for (methods::Rectangle, methods::BufferCell) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?, arg(a, 1)?))
    }
}

impl FromParams for (methods::Rectangle,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?,))
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
        Ok((arg(a, 0)?, arg(a, 1)?, arg(a, 2)?, arg(a, 3)?))
    }
}

impl FromParams for (PsValue,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        Ok((arg(a, 0)?,))
    }
}
