mod conversions;
mod error;
mod methods;
mod types;

pub use error::*;
pub use methods::*;
pub use types::*;

// Export spec-compliant utilities
pub use conversions::{RemoteHostMethodId, should_send_host_response};

use ironposh_psrp::{PipelineHostCall, PsValue};

#[derive(Debug, Clone)]
pub struct HostCallRequest {
    /// Type of the host call
    pub call_type: HostCallScope,
    /// Unique identifier for this host call
    pub call_id: i64,
    /// The host method identifier (enum value)
    pub method_id: i32,
    /// String representation of the method name
    pub method_name: String,
    /// Parameters for the method call as a list of values
    pub parameters: Vec<PsValue>,
}

impl HostCallRequest {
    pub fn new(
        call_type: HostCallScope,
        call_id: i64,
        method_id: i32,
        method_name: String,
        parameters: Vec<PsValue>,
    ) -> Self {
        Self {
            call_type,
            call_id,
            method_id,
            method_name,
            parameters,
        }
    }

    /// Extract the method call with typed parameters
    pub fn get_param(&self) -> Result<HostCallMethodWithParams, HostError> {
        HostCallMethodWithParams::try_from(self)
    }

    /// Submit the result and create a response
    pub fn submit_result(self, result: HostCallMethodReturn) -> HostCallResponse {
        // Extract method and delegate to the new submit method
        let method = match self.get_param() {
            Ok(method) => method,
            Err(error) => {
                // If we can't extract the method, create an error response
                return HostCallResponse {
                    call_scope: self.call_type,
                    call_id: self.call_id,
                    method_id: self.method_id,
                    method_name: self.method_name,
                    method_result: None,
                    method_exception: Some(PsValue::Primitive(
                        ironposh_psrp::PsPrimitiveValue::Str(error.to_string()),
                    )),
                };
            }
        };

        let (method_result, method_exception) = match method.submit(result) {
            Ok((result, exception)) => (result, exception),
            Err(error) => {
                // If submit fails, create an error response
                (
                    None,
                    Some(PsValue::Primitive(ironposh_psrp::PsPrimitiveValue::Str(
                        error.to_string(),
                    ))),
                )
            }
        };

        HostCallResponse {
            call_scope: self.call_type,
            call_id: self.call_id,
            method_id: self.method_id,
            method_name: self.method_name,
            method_result,
            method_exception,
        }
    }

    /// Convenience method to extract method and get a closure for submitting results
    /// Usage: let (method_result, method_exception) = self.get_method()?.submit(result)?;
    pub fn extract_method_and_submit(
        self,
        result: HostCallMethodReturn,
    ) -> Result<(Option<PsValue>, Option<PsValue>), HostError> {
        self.get_param()?.submit(result)
    }
}

#[derive(Debug, Clone)]
pub struct HostCallResponse {
    /// Type of the host call
    pub call_scope: HostCallScope,
    /// Unique identifier for this host call
    pub call_id: i64,
    /// The host method identifier (enum value)
    pub method_id: i32,
    /// String representation of the method name
    pub method_name: String,
    /// Optional return value from the method
    pub method_result: Option<PsValue>,
    /// Optional exception thrown by the method invocation
    pub method_exception: Option<PsValue>,
}

impl From<(&PipelineHostCall, HostCallScope)> for HostCallRequest {
    fn from((call, call_type): (&PipelineHostCall, HostCallScope)) -> Self {
        let PipelineHostCall {
            call_id,
            method_id,
            method_name,
            parameters,
        } = call;

        Self {
            call_type,
            call_id: *call_id,
            method_id: *method_id,
            method_name: method_name.to_string(),
            parameters: parameters.to_vec(),
        }
    }
}

impl From<HostCallRequest> for PipelineHostCall {
    fn from(val: HostCallRequest) -> Self {
        PipelineHostCall {
            call_id: val.call_id,
            method_id: val.method_id,
            method_name: val.method_name,
            parameters: val.parameters,
        }
    }
}
