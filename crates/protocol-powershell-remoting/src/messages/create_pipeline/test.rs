use uuid::Uuid;

use super::*;
use crate::{
    Fragmenter,
    ps_value::deserialize::{DeserializationContext, PsXmlDeserialize},
    ps_value::serialize::RefIdMap,
};

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
#[tracing_test::traced_test]
fn test_deserialize_real_create_pipeline() {
    let parsed = xml::parser::parse(REAL_CREATE_PIPELINE).unwrap();
    let root = parsed.root_element();

    let ps_value =
        PsValue::from_node_with_context(root, &mut DeserializationContext::default()).unwrap();

    if let PsValue::Object(complex_obj) = ps_value {
        let create_pipeline = CreatePipeline::try_from(complex_obj).unwrap();

        // Verify top-level CreatePipeline properties (lines 16, 37, 90)
        assert_eq!(create_pipeline.no_input, true, "NoInput should be true");
        assert_eq!(
            create_pipeline.add_to_history, false,
            "AddToHistory should be false"
        );
        assert_eq!(create_pipeline.is_nested, false, "IsNested should be false");

        // Verify ApartmentState (lines 17-26)
        assert_eq!(
            create_pipeline.apartment_state,
            ApartmentState::Unknown,
            "ApartmentState should be Unknown (2)"
        );

        // Verify RemoteStreamOptions (lines 27-36)
        assert_eq!(
            create_pipeline.remote_stream_options,
            RemoteStreamOptions::None,
            "RemoteStreamOptions should be None (0)"
        );

        // Verify HostInfo properties (lines 38-45)
        assert_eq!(
            create_pipeline.host_info.is_host_null, true,
            "_isHostNull should be true"
        );
        assert_eq!(
            create_pipeline.host_info.is_host_ui_null, true,
            "_isHostUINull should be true"
        );
        assert_eq!(
            create_pipeline.host_info.is_host_raw_ui_null, true,
            "_isHostRawUINull should be true"
        );
        assert_eq!(
            create_pipeline.host_info.use_runspace_host, true,
            "_useRunspaceHost should be true"
        );

        // Verify PowerShell pipeline properties (lines 46-88)
        assert_eq!(
            create_pipeline.power_shell.is_nested, false,
            "PowerShell.IsNested should be false"
        );
        assert_eq!(
            create_pipeline.power_shell.redirect_shell_error_output_pipe, true,
            "RedirectShellErrorOutputPipe should be true"
        );
        assert_eq!(
            create_pipeline.power_shell.history, "",
            "History should be empty string (Nil in XML)"
        );

        // Verify Commands array (line 53-83)
        assert_eq!(
            create_pipeline.power_shell.cmds.len(),
            1,
            "Should have exactly 1 command"
        );

        // Verify Command properties (lines 54-82)
        let cmd = &create_pipeline.power_shell.cmds[0];
        assert_eq!(
            cmd.cmd, r#"Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)""#,
            "Command text should match"
        );
        assert_eq!(cmd.is_script, true, "IsScript should be true");
        assert_eq!(
            cmd.use_local_scope, None,
            "UseLocalScope should be None (Nil in XML)"
        );
        assert_eq!(cmd.args.len(), 0, "Args should be empty list");

        // Verify merge properties all reference the same PipelineResultTypes::None (0)
        assert_eq!(
            cmd.merge_my_result,
            PipelineResultTypes::None,
            "MergeMyResult should be None (0)"
        );
        assert_eq!(
            cmd.merge_to_result,
            PipelineResultTypes::None,
            "MergeToResult should be None (0)"
        );
        assert_eq!(
            cmd.merge_previous_results,
            PipelineResultTypes::None,
            "MergePreviousResults should be None (0)"
        );
        assert_eq!(
            cmd.merge_error,
            PipelineResultTypes::None,
            "MergeError should be None (0)"
        );
        assert_eq!(
            cmd.merge_warning,
            PipelineResultTypes::None,
            "MergeWarning should be None (0)"
        );
        assert_eq!(
            cmd.merge_verbose,
            PipelineResultTypes::None,
            "MergeVerbose should be None (0)"
        );
        assert_eq!(
            cmd.merge_debug,
            PipelineResultTypes::None,
            "MergeDebug should be None (0)"
        );
        assert_eq!(
            cmd.merge_information,
            PipelineResultTypes::None,
            "MergeInformation should be None (0)"
        );
    } else {
        panic!("Expected ComplexObject");
    }
}
