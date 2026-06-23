use super::{HostCall, HostCallScope};
use ironposh_psrp::{PipelineHostCall, RemoteHostMethodId};
use uuid::Uuid;

#[test]
pub fn test_from_pipeline_host_call() {
    let pipeline_hostcall = PipelineHostCall {
        call_id: 1,
        method: RemoteHostMethodId::ReadLine,
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
        method: RemoteHostMethodId::WriteLine2,
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
pub fn test_invalid_method_id_rejected_on_parse() {
    // An unknown method id is now rejected when the wire `mi` enum is parsed
    // (the typed RemoteHostMethodId can't represent it), rather than at dispatch.
    use ironposh_psrp::ps_value::{
        ComplexObject, ComplexObjectContent, Properties, PsEnums, PsValue,
    };

    let mi = ComplexObject {
        type_def: None,
        to_string: None,
        content: ComplexObjectContent::PsEnums(PsEnums { value: 999 }),
        properties: Properties::new(),
    };
    let obj = ComplexObject::standard()
        .extended("ci", 1i64)
        .extended("mi", PsValue::Object(mi))
        .extended("mp", PsValue::from_array(vec![]))
        .build();

    assert!(PipelineHostCall::try_from(obj).is_err());
}

#[test]
pub fn test_from_pipeline_host_call_invalid_parameters() {
    use ironposh_psrp::PsValue;

    // Test ReadLine with incorrect parameters (should be empty)
    let pipeline_hostcall = PipelineHostCall {
        call_id: 1,
        method: RemoteHostMethodId::ReadLine,
        parameters: vec![PsValue::from("unexpected_param")], // ReadLine expects no params
    };

    let scope = HostCallScope::Pipeline {
        command_id: Uuid::new_v4(),
    };

    let result = HostCall::try_from_pipeline(scope, pipeline_hostcall);
    assert!(result.is_err());
}

#[test]
pub fn test_write_progress_tolerates_nil_fields() {
    // Regression: PowerShell's first WriteProgress ("Preparing modules for first
    // use.") sends `CurrentOperation` as an explicit `<Nil/>`. The derived
    // ProgressRecord maps that field to a non-optional `String` marked
    // `#[ps(default)]`; a present-but-Nil property must be treated like an absent
    // one (→ default), not fed to `String::from_ps_value` (which rejects Nil).
    use ironposh_psrp::ps_value::{
        ComplexObject, ComplexObjectContent, PsEnums, PsPrimitiveValue, PsValue,
    };

    // The nested ProgressRecordType enum object (= "Completed").
    let progress_type = ComplexObject {
        type_def: None,
        to_string: Some("Completed".to_string()),
        content: ComplexObjectContent::PsEnums(PsEnums { value: 1 }),
        properties: ironposh_psrp::ps_value::Properties::new(),
    };

    let record = ComplexObject::standard()
        .extended("Activity", "Preparing modules for first use.")
        .extended("ActivityId", 0i32)
        .extended("CurrentOperation", PsValue::Primitive(PsPrimitiveValue::Nil))
        .extended("ParentActivityId", -1i32)
        .extended("PercentComplete", -1i32)
        .extended("SecondsRemaining", -1i32)
        .extended("StatusDescription", " ")
        .extended("Type", PsValue::Object(progress_type))
        .build();

    let pipeline_hostcall = PipelineHostCall {
        call_id: -100,
        method: RemoteHostMethodId::WriteProgress,
        parameters: vec![PsValue::from(1i64), PsValue::Object(record)],
    };

    let scope = HostCallScope::Pipeline {
        command_id: Uuid::new_v4(),
    };

    let host_call = HostCall::try_from_pipeline(scope, pipeline_hostcall)
        .expect("WriteProgress with a Nil CurrentOperation must parse");

    match &host_call {
        HostCall::WriteProgress { transport } => {
            let (source_id, progress) = &transport.params;
            assert_eq!(*source_id, 1);
            assert_eq!(progress.activity, "Preparing modules for first use.");
            assert_eq!(progress.current_operation, String::new()); // Nil → default
            assert_eq!(progress.percent_complete, -1);
        }
        _ => panic!("Expected WriteProgress variant"),
    }
}

#[test]
pub fn test_from_pipeline_host_call_set_should_exit() {
    use ironposh_psrp::PsValue;

    // Test SetShouldExit which takes an i32 parameter
    let pipeline_hostcall = PipelineHostCall {
        call_id: 123,
        method: RemoteHostMethodId::SetShouldExit,
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
