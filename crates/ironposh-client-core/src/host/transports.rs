use super::{
    conversions::should_send_host_response,
    traits::{Method, ToPs},
    types::HostCallScope,
};
use core::marker::PhantomData;
use ironposh_psrp::PipelineHostResponse;

/// Transport wraps method parameters and provides typed result submission
#[derive(Debug)]
pub struct Transport<M: Method> {
    pub scope: HostCallScope,
    pub call_id: i64,
    pub params: M::Params,
    pub(crate) _m: PhantomData<M>,
}

impl<M: Method> Transport<M> {
    pub fn new(scope: HostCallScope, call_id: i64, params: M::Params) -> Self {
        Self {
            scope,
            call_id,
            params,
            _m: PhantomData,
        }
    }

    pub fn into_parts(self) -> (M::Params, ResultTransport<M>) {
        (
            self.params,
            ResultTransport {
                scope: self.scope,
                call_id: self.call_id,
                _m: PhantomData,
            },
        )
    }
}

/// Result transport handles typed return values and creates pipeline responses
pub struct ResultTransport<M: Method> {
    scope: HostCallScope,
    call_id: i64,
    _m: PhantomData<M>,
}

/// What gets passed back to the session
#[derive(Debug)]
pub enum Submission {
    Send(PipelineHostResponse),
    NoSend,
}

impl<M: Method> ResultTransport<M> {
    /// Accept a result - automatically determines if response should be sent based on method
    pub fn accept_result(self, v: M::Return) -> Submission
    where
        M::Return: ToPs,
    {
        if should_send_host_response(M::ID) {
            Submission::Send(PipelineHostResponse {
                call_id: self.call_id,
                method_id: M::ID as i32,
                method_name: format!("{:?}", M::ID),
                method_result: <M::Return as ToPs>::to_ps(v),
                method_exception: None,
            })
        } else {
            Submission::NoSend
        }
    }
}
