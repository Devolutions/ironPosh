#[cfg(test)]
mod tests {
    use crate::messages::create_pipeline::Command;
    use crate::ps_value::{
        ComplexObject, PsValue,
        deserialize::{DeserializationContext, PsXmlDeserialize},
    };

    const XML_TEMPLATE: &str = r#"<Obj RefId="0">
    <MS>
        <Obj N="PowerShell" RefId="1">
            <MS>
                <Obj N="Cmds" RefId="2">
                    <TN RefId="0">
                        <T>System.Collections.Generic.List`1[[System.Management.Automation.PSObject, System.Management.Automation, Version=3.0.0.0, Culture=neutral, PublicKeyToken=31bf3856ad364e35]]</T>
                        <T>System.Object</T>
                    </TN>
                    <LST>
                        <Obj RefId="3">
                            <MS>
                                <S N="Cmd">Invoke-expression</S>
                                <B N="IsScript">false</B>
                                <Nil N="UseLocalScope" />
                                <Obj N="MergeMyResult" RefId="4">
                                    <TN RefId="1">
                                        <T>System.Management.Automation.Runspaces.PipelineResultTypes</T>
                                        <T>System.Enum</T>
                                        <T>System.ValueType</T>
                                        <T>System.Object</T>
                                    </TN>
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeToResult" RefId="5">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergePreviousResults" RefId="6">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeError" RefId="7">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeWarning" RefId="8">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeVerbose" RefId="9">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeDebug" RefId="10">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="Args" RefId="11">
                                    <TNRef RefId="0" />
                                    <LST>
                                        <Obj RefId="12">
                                            <MS>
                                                <S N="N">-Command</S>
                                                <Nil N="V" />
                                            </MS>
                                        </Obj>
                                        <Obj RefId="13">
                                            <MS>
                                                <Nil N="N" />
                                                <S N="V">ls</S>
                                            </MS>
                                        </Obj>
                                    </LST>
                                </Obj>
                            </MS>
                        </Obj>
                        <Obj RefId="13">
                            <MS>
                                <S N="Cmd">Out-String</S>
                                <B N="IsScript">false</B>
                                <B N="UseLocalScope">true</B>
                                <Obj N="MergeMyResult" RefId="14">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeToResult" RefId="15">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergePreviousResults" RefId="16">
                                    <TNRef RefId="1" />
                                    <ToString>Warning</ToString>
                                    <I32>3</I32>
                                </Obj>
                                <Obj N="MergeError" RefId="17">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeWarning" RefId="18">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeVerbose" RefId="19">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeDebug" RefId="20">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="MergeInformation" RefId="21">
                                    <TNRef RefId="1" />
                                    <ToString>None</ToString>
                                    <I32>0</I32>
                                </Obj>
                                <Obj N="Args" RefId="22">
                                    <TNRef RefId="0" />
                                    <LST>
                                        <Obj RefId="23">
                                            <MS>
                                                <S N="N">-stream</S>
                                                <Nil N="V" />
                                            </MS>
                                        </Obj>
                                    </LST>
                                </Obj>
                            </MS>
                        </Obj>
                    </LST>
                </Obj>
                <B N="IsNested">false</B>
                <Nil N="History" />
                <B N="RedirectShellErrorOutputPipe">true</B>
            </MS>
        </Obj>
        <B N="NoInput">true</B>
        <Obj N="ApartmentState" RefId="24">
            <TN RefId="2">
                <T>System.Threading.ApartmentState</T>
                <T>System.Enum</T>
                <T>System.ValueType</T>
                <T>System.Object</T>
            </TN>
            <ToString>Unknown</ToString>
            <I32>2</I32>
        </Obj>
        <Obj N="RemoteStreamOptions" RefId="25">
            <TN RefId="3">
                <T>System.Management.Automation.RemoteStreamOptions</T>
                <T>System.Enum</T>
                <T>System.ValueType</T>
                <T>System.Object</T>
            </TN>
            <ToString>0</ToString>
            <I32>0</I32>
        </Obj>
        <B N="AddToHistory">true</B>
        <Obj N="HostInfo" RefId="26">
            <MS>
                <B N="_isHostNull">true</B>
                <B N="_isHostUINull">true</B>
                <B N="_isHostRawUINull">true</B>
                <B N="_useRunspaceHost">true</B>
            </MS>
        </Obj>
        <B N="IsNested">false</B>
    </MS>
</Obj>"#;

    #[test]
    fn test_round_trip_create_pipeline() {
        use crate::messages::create_pipeline::CreatePipeline;

        // Parse the XML to CreatePipeline
        let parsed = ironposh_xml::parser::parse(XML_TEMPLATE).expect("Failed to parse XML");
        let root = parsed.root_element();

        let mut context = DeserializationContext::default();
        let ps_value = PsValue::from_node_with_context(root, &mut context)
            .expect("Failed to parse XML to PsValue");

        let original_obj = ps_value.as_object().expect("Expected complex object");

        // Convert to CreatePipeline
        let create_pipeline = CreatePipeline::try_from(original_obj.clone())
            .expect("Failed to convert to CreatePipeline");

        println!("Parsed CreatePipeline: {create_pipeline:#?}");

        // Convert back to ComplexObject
        let recreated_obj = ComplexObject::from(create_pipeline);

        let xml = PsValue::Object(recreated_obj.clone())
            .to_element_as_root()
            .unwrap()
            .to_xml_string()
            .unwrap();

        println!("Recreated XML:\n{xml}");

        // Test round-trip
        assert_eq!(
            original_obj, &recreated_obj,
            "Round-trip failed: original != recreated"
        );
    }

    #[test]
    fn test_extract_and_round_trip_commands() {
        // Parse XML and extract individual commands
        let parsed = ironposh_xml::parser::parse(XML_TEMPLATE).expect("Failed to parse XML");
        let root = parsed.root_element();

        let mut context = DeserializationContext::default();
        let ps_value = PsValue::from_node_with_context(root, &mut context)
            .expect("Failed to parse XML to PsValue");

        let top_level_obj = ps_value.as_object().expect("Expected complex object");

        // Navigate to PowerShell -> Cmds -> Command list
        let powershell_prop = top_level_obj
            .extended_properties
            .get("PowerShell")
            .expect("Missing PowerShell property");

        let powershell_obj = powershell_prop
            .value
            .as_object()
            .expect("PowerShell should be an object");

        let cmds_prop = powershell_obj
            .extended_properties
            .get("Cmds")
            .expect("Missing Cmds property");

        let cmds_obj = cmds_prop
            .value
            .as_object()
            .expect("Cmds should be an object");

        if let crate::ps_value::ComplexObjectContent::Container(crate::ps_value::Container::List(
            cmd_list,
        )) = &cmds_obj.content
        {
            println!("Found {} commands", cmd_list.len());

            for (i, cmd_value) in cmd_list.iter().enumerate() {
                if let PsValue::Object(cmd_obj) = cmd_value {
                    println!("Testing command {i}");

                    // Test round-trip for individual command
                    let command = Command::try_from(cmd_obj.clone())
                        .unwrap_or_else(|_| panic!("Failed to parse command {i}"));

                    println!("Command {i}: {command:#?}");

                    let recreated_obj = ComplexObject::from(command);

                    assert_eq!(cmd_obj, &recreated_obj, "Command {i} round-trip failed");
                }
            }
        } else {
            panic!("Commands should be a list");
        }
    }
}
