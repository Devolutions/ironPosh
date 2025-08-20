use crate::messages::{
    ComplexObject, PipelineHostCall, PsValue,
    deserialize::{DeserializationContext, PsXmlDeserialize},
};

const PIPELINE_HOST_CALL: &'static str = r#"
<Obj RefId="0"><MS><I64 N="ci">-100</I64><Obj N="mi" RefId="1"><TN RefId="0"><T>System.Management.Automation.Remoting.RemoteHostMethoodId</T><T>System.Enum</T><T>System.ValueType</T><T>System.Object</T></TN><ToString>WriteProgress</ToString><I32>20</I32></Obj><Obj N="mp" RefId="2"><TN RefId="1"><T>System.Collections.ArrayList</T><T>System.Object</T></TN><LST><I64>3</I64><Obj RefId="3"><MS><S N="Activity">Preparing modules for first use.</S><I32 N="ActivityId">0</I32><S N="StatusDescription"> </S><Nil N="CurrentOperation" /><I32 N="ParentActivityId">-1</I32><I32 N="PercentComplete">-1</I32><Obj N="Type" RefId="4"><TN RefId="2"><T>System.Management.Automation.ProgressRecordType</T><T>System.Enum</T><T>System.ValueType</T><T>System.Object</T></TN><ToString>Completed</ToString><I32>1</I32></Obj><I32 N="SecondsRemaining">-1</I32></MS></Obj></LST></Obj></MS></Obj>
"#;

#[test]
fn test_parse_real_pipeline_host_call() {
    // Parse the XML and deserialize
    let parsed = xml::parser::parse(PIPELINE_HOST_CALL).expect("Failed to parse XML");
    let root = parsed.root_element();
    let mut context = DeserializationContext::new();
    let complex_obj =
        ComplexObject::from_node_with_context(root, &mut context).expect("Failed to deserialize");

    println!("Complex Object: {:#?}", complex_obj);

    // Debug the method identifier structure
    if let Some(mi_prop) = complex_obj.extended_properties.get("mi") {
        if let PsValue::Object(mi_obj) = &mi_prop.value {
            println!("Method identifier object content: {:?}", mi_obj.content);
            println!("Method identifier to_string: {:?}", mi_obj.to_string);
        }
    }

    let pipeline_host_call =
        PipelineHostCall::try_from(complex_obj).expect("Failed to convert to PipelineHostCall");

    println!("PipelineHostCall: {:#?}", pipeline_host_call);

    // Verify the parsed values
    assert_eq!(pipeline_host_call.call_id, -100);
    assert_eq!(pipeline_host_call.method_id, 20);
    assert_eq!(pipeline_host_call.method_name, "WriteProgress");
    assert_eq!(pipeline_host_call.parameters.len(), 2); // I64(3) and the progress record object
}
