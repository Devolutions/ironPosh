use super::{HostCall, HostCallScope};
use ironposh_psrp::PipelineHostCall;
use uuid::Uuid;

#[test]
pub fn test_from_pipeline_host_call() {
    let pipeline_hostcall = PipelineHostCall {
        call_id: 1,
        method_id: 11,
        method_name: "ReadLine".to_string(),
        parameters: vec![],
    };

    // Test conversion from PipelineHostCall to HostCall
    let scope = HostCallScope::Pipeline {
        command_id: Uuid::new_v4(),
    };

    let host_call = HostCall::try_from_pipeline(scope.clone(), pipeline_hostcall).unwrap();

    // Verify the conversion was successful
    match &host_call {
        HostCall::ReadLine { transport } => {
            assert_eq!(transport.call_id, 1);
            assert_eq!(transport.scope, scope);
            // Parameters should be empty tuple for ReadLine
            assert_eq!(transport.params, ());
        }
        _ => panic!("Expected ReadLine variant"),
    }

    // Test accessor methods
    assert_eq!(host_call.call_id(), 1);
    assert_eq!(host_call.method_name(), "ReadLine");
    assert_eq!(host_call.method_id(), 11);
    assert_eq!(host_call.scope(), scope);
}

#[test]
pub fn test_from_pipeline_host_call_with_parameters() {
    use ironposh_psrp::PsValue;

    // Test WriteLine2 which takes a String parameter
    let pipeline_hostcall = PipelineHostCall {
        call_id: 42,
        method_id: 16, // WriteLine2
        method_name: "WriteLine2".to_string(),
        parameters: vec![PsValue::from("Hello, World!")],
    };

    let scope = HostCallScope::RunspacePool;

    let host_call = HostCall::try_from_pipeline(scope.clone(), pipeline_hostcall).unwrap();

    match &host_call {
        HostCall::WriteLine2 { transport } => {
            assert_eq!(transport.call_id, 42);
            assert_eq!(transport.scope, scope);
            // Parameters should be a String tuple
            assert_eq!(transport.params, ("Hello, World!".to_string(),));
        }
        _ => panic!("Expected WriteLine2 variant"),
    }

    assert_eq!(host_call.call_id(), 42);
    assert_eq!(host_call.method_name(), "WriteLine2");
    assert_eq!(host_call.method_id(), 16);
    assert_eq!(host_call.scope(), scope);
}

#[test]
pub fn test_from_pipeline_host_call_invalid_method_id() {
    let pipeline_hostcall = PipelineHostCall {
        call_id: 1,
        method_id: 999, // Invalid method ID
        method_name: "InvalidMethod".to_string(),
        parameters: vec![],
    };

    let scope = HostCallScope::Pipeline {
        command_id: Uuid::new_v4(),
    };

    let result = HostCall::try_from_pipeline(scope, pipeline_hostcall);
    assert!(result.is_err());
}

#[test]
pub fn test_from_pipeline_host_call_invalid_parameters() {
    use ironposh_psrp::PsValue;

    // Test ReadLine with incorrect parameters (should be empty)
    let pipeline_hostcall = PipelineHostCall {
        call_id: 1,
        method_id: 11, // ReadLine
        method_name: "ReadLine".to_string(),
        parameters: vec![PsValue::from("unexpected_param")], // ReadLine expects no params
    };

    let scope = HostCallScope::Pipeline {
        command_id: Uuid::new_v4(),
    };

    let result = HostCall::try_from_pipeline(scope, pipeline_hostcall);
    assert!(result.is_err());
}

#[test]
pub fn test_from_pipeline_host_call_set_should_exit() {
    use ironposh_psrp::PsValue;

    // Test SetShouldExit which takes an i32 parameter
    let pipeline_hostcall = PipelineHostCall {
        call_id: 123,
        method_id: 6, // SetShouldExit
        method_name: "SetShouldExit".to_string(),
        parameters: vec![PsValue::from(42i32)],
    };

    let scope = HostCallScope::RunspacePool;

    let host_call = HostCall::try_from_pipeline(scope.clone(), pipeline_hostcall).unwrap();

    match &host_call {
        HostCall::SetShouldExit { transport } => {
            assert_eq!(transport.call_id, 123);
            assert_eq!(transport.scope, scope);
            // Parameters should be an i32 tuple
            assert_eq!(transport.params, (42,));
        }
        _ => panic!("Expected SetShouldExit variant"),
    }

    assert_eq!(host_call.call_id(), 123);
    assert_eq!(host_call.method_name(), "SetShouldExit");
    assert_eq!(host_call.method_id(), 6);
    assert_eq!(host_call.scope(), scope);
}
