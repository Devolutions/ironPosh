use super::*;
use crate::ps_value::deserialize::{DeserializationContext, PsXmlDeserialize};

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
				<Obj N="_hostDefaultData">
					<MS>
						<Obj N="data">
							<TN RefId="10">
								<T>System.Collections.Hashtable</T>
								<T>System.Object</T>
							</TN>
							<DCT>
								<En>
									<I32 N="Key">0</I32>
									<Obj N="Value"><MS><S N="T">System.ConsoleColor</S><I32 N="V">7</I32></MS></Obj>
								</En>
								<En>
									<I32 N="Key">1</I32>
									<Obj N="Value"><MS><S N="T">System.ConsoleColor</S><I32 N="V">0</I32></MS></Obj>
								</En>
								<En>
									<I32 N="Key">2</I32>
									<Obj N="Value"><MS><S N="T">System.Management.Automation.Host.Coordinates</S><Obj N="V"><MS><I32 N="x">0</I32><I32 N="y">0</I32></MS></Obj></MS></Obj>
								</En>
								<En>
									<I32 N="Key">3</I32>
									<Obj N="Value"><MS><S N="T">System.Management.Automation.Host.Coordinates</S><Obj N="V"><MS><I32 N="x">0</I32><I32 N="y">0</I32></MS></Obj></MS></Obj>
								</En>
								<En>
									<I32 N="Key">4</I32>
									<Obj N="Value"><MS><S N="T">System.Int32</S><I32 N="V">25</I32></MS></Obj>
								</En>
								<En>
									<I32 N="Key">5</I32>
									<Obj N="Value"><MS><S N="T">System.Management.Automation.Host.Size</S><Obj N="V"><MS><I32 N="width">120</I32><I32 N="height">3000</I32></MS></Obj></MS></Obj>
								</En>
								<En>
									<I32 N="Key">6</I32>
									<Obj N="Value"><MS><S N="T">System.Management.Automation.Host.Size</S><Obj N="V"><MS><I32 N="width">120</I32><I32 N="height">50</I32></MS></Obj></MS></Obj>
								</En>
								<En>
									<I32 N="Key">7</I32>
									<Obj N="Value"><MS><S N="T">System.Management.Automation.Host.Size</S><Obj N="V"><MS><I32 N="width">120</I32><I32 N="height">50</I32></MS></Obj></MS></Obj>
								</En>
								<En>
									<I32 N="Key">8</I32>
									<Obj N="Value"><MS><S N="T">System.Management.Automation.Host.Size</S><Obj N="V"><MS><I32 N="width">120</I32><I32 N="height">50</I32></MS></Obj></MS></Obj>
								</En>
								<En>
									<I32 N="Key">9</I32>
									<Obj N="Value"><MS><S N="T">System.String</S><S N="V">PowerShell</S></MS></Obj>
								</En>
								<En>
									<I32 N="Key">10</I32>
									<Obj N="Value"><MS><S N="T">System.String</S><S N="V">en-US</S></MS></Obj>
								</En>
								<En>
									<I32 N="Key">11</I32>
									<Obj N="Value"><MS><S N="T">System.String</S><S N="V">en-US</S></MS></Obj>
								</En>
							</DCT>
						</Obj>
					</MS>
				</Obj>
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
								<S N="Cmd">Invoke-Expression</S>
								<Obj RefId="7" N="Args">
									<TNRef RefId="3" />
									<LST>
										<Obj RefId="8">
											<MS>
												<S N="N">Command</S>
												<S N="V">ls</S>
											</MS>
										</Obj>
									</LST>
								</Obj>
								<B N="IsScript">false</B>
								<Nil N="UseLocalScope" />
								<Obj RefId="9" N="MergeMyResult">
									<I32>0</I32>
									<TN RefId="4">
										<T>System.Management.Automation.Runspaces.PipelineResultTypes</T>
										<T>System.Enum</T>
										<T>System.ValueType</T>
										<T>System.Object</T>
									</TN>
									<ToString>None</ToString>
								</Obj>
								<Ref RefId="9" N="MergeToResult" />
								<Ref RefId="9" N="MergePreviousResults" />
								<Ref RefId="9" N="MergeError" />
								<Ref RefId="9" N="MergeWarning" />
								<Ref RefId="9" N="MergeVerbose" />
								<Ref RefId="9" N="MergeDebug" />
								<Ref RefId="9" N="MergeInformation" />
							</MS>
							<ToString>Invoke-Expression</ToString>
						</Obj>
						<Obj RefId="10">
							<MS>
								<S N="Cmd">Out-String</S>
								<Obj RefId="11" N="Args">
									<TNRef RefId="3" />
									<LST>
										<Obj RefId="12">
											<MS>
												<S N="N">Stream</S>
												<B N="V">true</B>
											</MS>
										</Obj>
									</LST>
								</Obj>
								<B N="IsScript">false</B>
								<Nil N="UseLocalScope" />
								<Ref RefId="9" N="MergeMyResult" />
								<Ref RefId="9" N="MergeToResult" />
								<Ref RefId="9" N="MergePreviousResults" />
								<Ref RefId="9" N="MergeError" />
								<Ref RefId="9" N="MergeWarning" />
								<Ref RefId="9" N="MergeVerbose" />
								<Ref RefId="9" N="MergeDebug" />
								<Ref RefId="9" N="MergeInformation" />
							</MS>
							<ToString>Out-String</ToString>
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
    let parsed = ironposh_xml::parser::parse(REAL_CREATE_PIPELINE).unwrap();
    let root = parsed.root_element();

    let ps_value =
        PsValue::from_node_with_context(root, &mut DeserializationContext::default()).unwrap();

    if let PsValue::Object(complex_obj) = ps_value {
        let create_pipeline = CreatePipeline::try_from(complex_obj).unwrap();

        // Verify top-level CreatePipeline properties (lines 16, 37, 90)
        assert!(create_pipeline.no_input, "NoInput should be true");
        assert!(
            !create_pipeline.add_to_history,
            "AddToHistory should be false"
        );
        assert!(!create_pipeline.is_nested, "IsNested should be false");

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
        assert!(
            create_pipeline.host_info.is_host_null,
            "_isHostNull should be true"
        );
        assert!(
            create_pipeline.host_info.is_host_ui_null,
            "_isHostUINull should be true"
        );
        assert!(
            create_pipeline.host_info.is_host_raw_ui_null,
            "_isHostRawUINull should be true"
        );
        assert!(
            create_pipeline.host_info.use_runspace_host,
            "_useRunspaceHost should be true"
        );

        // Verify PowerShell pipeline properties (lines 46-88)
        assert!(
            !create_pipeline.pipeline.is_nested,
            "PowerShell.IsNested should be false"
        );
        assert!(
            create_pipeline.pipeline.redirect_shell_error_output_pipe,
            "RedirectShellErrorOutputPipe should be true"
        );
        assert_eq!(
            create_pipeline.pipeline.history, "",
            "History should be empty string (Nil in XML)"
        );

        // Verify Commands array - the XML has 2 commands: Invoke-Expression and Out-String
        assert_eq!(
            create_pipeline.pipeline.cmds.len(),
            2,
            "Should have exactly 2 commands"
        );

        // Verify first command (Invoke-Expression)
        let cmd1 = &create_pipeline.pipeline.cmds[0];
        assert_eq!(
            cmd1.cmd, "Invoke-Expression",
            "First command should be Invoke-Expression"
        );
        assert!(
            !cmd1.is_script,
            "IsScript should be false for Invoke-Expression"
        );
        assert_eq!(
            cmd1.use_local_scope, None,
            "UseLocalScope should be None (Nil in XML)"
        );
        assert_eq!(
            cmd1.args.len(),
            1,
            "Invoke-Expression should have 1 argument"
        );

        // Verify second command (Out-String)
        let cmd2 = &create_pipeline.pipeline.cmds[1];
        assert_eq!(
            cmd2.cmd, "Out-String",
            "Second command should be Out-String"
        );
        assert!(!cmd2.is_script, "IsScript should be false for Out-String");
        assert_eq!(cmd2.args.len(), 1, "Out-String should have 1 argument");

        // Verify merge properties all reference the same PipelineResultTypes::None (0) for first command
        assert_eq!(
            cmd1.merge_my_result,
            PipelineResultTypes::None,
            "MergeMyResult should be None (0)"
        );
        assert_eq!(
            cmd1.merge_to_result,
            PipelineResultTypes::None,
            "MergeToResult should be None (0)"
        );
        assert_eq!(
            cmd1.merge_previous_results,
            PipelineResultTypes::None,
            "MergePreviousResults should be None (0)"
        );
        assert_eq!(
            cmd1.merge_error,
            PipelineResultTypes::None,
            "MergeError should be None (0)"
        );
        assert_eq!(
            cmd1.merge_warning,
            PipelineResultTypes::None,
            "MergeWarning should be None (0)"
        );
        assert_eq!(
            cmd1.merge_verbose,
            PipelineResultTypes::None,
            "MergeVerbose should be None (0)"
        );
        assert_eq!(
            cmd1.merge_debug,
            PipelineResultTypes::None,
            "MergeDebug should be None (0)"
        );
        assert_eq!(
            cmd1.merge_information,
            PipelineResultTypes::None,
            "MergeInformation should be None (0)"
        );
    } else {
        panic!("Expected ComplexObject");
    }
}
