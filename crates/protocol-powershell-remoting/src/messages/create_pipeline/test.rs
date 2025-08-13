
use super::*;
use crate::deserialize::{DeserializationContext, PsXmlDeserialize};

const REAL_CREATE_PIPELINE: &str = r#"
<Obj RefId="0">
   <TN RefId="0">
      <T>System.Object</T>
   </TN>
   <MS>
      <B N="NoInput">true</B>
      <Obj RefId="1" N="ApartmentState">
         <I32>2</I32>
         <TN RefId="1">
            <T>System.Threading.ApartmentState</T>
            <T>System.Enum</T>
            <T>System.ValueType</T>
            <T>System.Object</T>
         </TN>
         <ToString>Unknown</ToString>
      </Obj>
      <Obj RefId="2" N="RemoteStreamOptions">
         <I32>0</I32>
         <TN RefId="2">
            <T>System.Management.Automation.RemoteStreamOptions</T>
            <T>System.Enum</T>
            <T>System.ValueType</T>
            <T>System.Object</T>
         </TN>
         <ToString>None</ToString>
      </Obj>
      <B N="AddToHistory">false</B>
      <Obj RefId="3" N="HostInfo">
         <MS>
            <B N="_isHostNull">true</B>
            <B N="_isHostUINull">true</B>
            <B N="_isHostRawUINull">true</B>
            <B N="_useRunspaceHost">true</B>
         </MS>
      </Obj>
      <Obj RefId="4" N="PowerShell">
         <MS>
            <Obj RefId="5" N="Cmds">
               <TN RefId="3">
                  <T>System.Collections.ArrayList</T>
                  <T>System.Object</T>
               </TN>
               <LST>
                  <Obj RefId="6">
                     <MS>
                        <S N="Cmd">Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)"</S>
                        <Obj RefId="7" N="Args">
                           <TNRef RefId="3" />
                           <LST />
                        </Obj>
                        <B N="IsScript">true</B>
                        <Nil N="UseLocalScope" />
                        <Obj RefId="8" N="MergeMyResult">
                           <I32>0</I32>
                           <TN RefId="4">
                              <T>System.Management.Automation.Runspaces.PipelineResultTypes</T>
                              <T>System.Enum</T>
                              <T>System.ValueType</T>
                              <T>System.Object</T>
                           </TN>
                           <ToString>None</ToString>
                        </Obj>
                        <Ref RefId="8" N="MergeToResult" />
                        <Ref RefId="8" N="MergePreviousResults" />
                        <Ref RefId="8" N="MergeError" />
                        <Ref RefId="8" N="MergeWarning" />
                        <Ref RefId="8" N="MergeVerbose" />
                        <Ref RefId="8" N="MergeDebug" />
                        <Ref RefId="8" N="MergeInformation" />
                     </MS>
                     <ToString>Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)"</ToString>
                  </Obj>
               </LST>
            </Obj>
            <B N="IsNested">false</B>
            <Nil N="History" />
            <B N="RedirectShellErrorOutputPipe">true</B>
         </MS>
      </Obj>
      <B N="IsNested">false</B>
   </MS>
</Obj>
    "#;

#[test]
fn test_create_pipeline_simple_command() {
    let pipeline = CreatePipeline::simple_command("Get-Process");

    assert_eq!(pipeline.power_shell.cmds.len(), 1);
    assert_eq!(pipeline.power_shell.cmds[0].cmd, "Get-Process");
    assert!(!pipeline.power_shell.cmds[0].is_script);
}

#[test]
fn test_create_pipeline_script_command() {
    let script = r#"Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)""#;
    let pipeline = CreatePipeline::script_command(script);

    assert_eq!(pipeline.power_shell.cmds.len(), 1);
    assert_eq!(pipeline.power_shell.cmds[0].cmd, script);
    assert!(pipeline.power_shell.cmds[0].is_script);
}

#[test]
fn test_message_type() {
    let pipeline = CreatePipeline::simple_command("Test");
    assert_eq!(pipeline.message_type().value(), 0x00021006);
}

#[test]
fn test_deserialize_real_create_pipeline() {
    let parsed = xml::parser::parse(REAL_CREATE_PIPELINE).unwrap();
    let root = parsed.root_element();

    let ps_value =
        PsValue::from_node_with_context(root, &mut DeserializationContext::default()).unwrap();

    if let PsValue::Object(complex_obj) = ps_value {
        let create_pipeline = CreatePipeline::try_from(complex_obj).unwrap();

        // Verify the parsed values match the XML
        assert_eq!(create_pipeline.no_input, true);
        assert_eq!(create_pipeline.apartment_state, ApartmentState::Unknown);
        assert_eq!(
            create_pipeline.remote_stream_options,
            RemoteStreamOptions::None
        );
        assert_eq!(create_pipeline.add_to_history, false);
        assert_eq!(create_pipeline.is_nested, false);

        // Verify host info
        assert_eq!(create_pipeline.host_info.is_host_null, true);
        assert_eq!(create_pipeline.host_info.is_host_ui_null, true);
        assert_eq!(create_pipeline.host_info.is_host_raw_ui_null, true);
        assert_eq!(create_pipeline.host_info.use_runspace_host, true);

        // Verify pipeline
        assert_eq!(create_pipeline.power_shell.is_nested, false);
        assert_eq!(
            create_pipeline.power_shell.redirect_shell_error_output_pipe,
            true
        );
        assert_eq!(create_pipeline.power_shell.cmds.len(), 1);

        // Verify the command
        let cmd = &create_pipeline.power_shell.cmds[0];
        assert_eq!(
            cmd.cmd,
            r#"Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)""#
        );
        assert_eq!(cmd.is_script, true);
        assert_eq!(cmd.use_local_scope, None);
        assert_eq!(cmd.args.len(), 0);
    } else {
        panic!("Expected ComplexObject");
    }
}

#[test]
fn test_roundtrip_serialization() {
    let script = r#"Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)""#;
    let original = CreatePipeline::script_command(script);

    // Serialize to ComplexObject
    let complex_obj = ComplexObject::from(original.clone());

    // Deserialize back
    let roundtrip = CreatePipeline::try_from(complex_obj).unwrap();

    // Verify they're equal
    assert_eq!(original.no_input, roundtrip.no_input);
    assert_eq!(original.apartment_state, roundtrip.apartment_state);
    assert_eq!(
        original.remote_stream_options,
        roundtrip.remote_stream_options
    );
    assert_eq!(original.add_to_history, roundtrip.add_to_history);
    assert_eq!(original.is_nested, roundtrip.is_nested);
    assert_eq!(
        original.power_shell.cmds[0].cmd,
        roundtrip.power_shell.cmds[0].cmd
    );
    assert_eq!(
        original.power_shell.cmds[0].is_script,
        roundtrip.power_shell.cmds[0].is_script
    );
}
