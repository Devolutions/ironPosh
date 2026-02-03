use crate::{
    CommandCompletion,
    ps_value::{
        PsValue,
        deserialize::{DeserializationContext, PsXmlDeserialize},
    },
};

const COMMAND_COMPLETION_GET_SER: &str = include_str!("resources/command_completion_get_ser.xml");

#[test]
fn parse_command_completion_from_tab_expansion2() {
    let parsed =
        ironposh_xml::parser::parse(COMMAND_COMPLETION_GET_SER).expect("Failed to parse XML");
    let root = parsed.root_element();

    let mut context = DeserializationContext::default();
    let ps_value =
        PsValue::from_node_with_context(root, &mut context).expect("Failed to parse PsValue");

    let completion =
        CommandCompletion::try_from(&ps_value).expect("Failed to parse CommandCompletion");

    assert_eq!(completion.replacement_index, 0);
    assert_eq!(completion.replacement_length, 7);
    assert_eq!(completion.current_match_index, -1);
    assert!(
        !completion.completion_matches.is_empty(),
        "Expected at least one completion match"
    );

    // `Get-Ser` should produce command completions starting with "Get-Ser".
    assert!(
        completion
            .completion_matches
            .iter()
            .any(|m| m.completion_text.starts_with("Get-Ser")),
        "Expected at least one completion starting with Get-Ser"
    );
}
