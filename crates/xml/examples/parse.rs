const SOAP: &str = r#"
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
            xmlns:wsman="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd">
  <s:Header>
    <wsa:To s:mustUnderstand="true">http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</wsa:To>
    <wsa:ReplyTo>
      <wsa:Address>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</wsa:Address>
    </wsa:ReplyTo>
    <wsa:Action>http://schemas.xmlsoap.org/ws/2004/09/transfer/Get</wsa:Action>
    <wsa:MessageID>uuid:12345678-1234-1234-1234-1234567890ab</wsa:MessageID>
    <wsman:ResourceURI>http://schemas.microsoft.com/wbem/wsman/1/wmi/root/cimv2/Win32_OperatingSystem</wsman:ResourceURI>
    <wsman:OperationTimeout>PT60.000S</wsman:OperationTimeout>
    <wsman:Locale xml:lang="en-US" />
    <wsman:OptionSet>
      <wsman:Option Name="OptimizeEnumeration">true</wsman:Option>
    </wsman:OptionSet>
    <wsman:MaxEnvelopeSize s:mustUnderstand="true">153600</wsman:MaxEnvelopeSize>
  </s:Header>
  <s:Body />
</s:Envelope>
"#;

pub fn main() {
    let parsed = xml::parser::parse(SOAP).expect("Failed to parse XML");
    // println!("{:#?}", parsed);

    let root = parsed.root().first_child().unwrap();
    // println!("Root: {:#?}", root);
    let header = root
        .children()
        .find(|n| n.tag_name().name() == "Header")
        .unwrap();
    // println!("Header: {:#?}", header);

    let wsa_node = header
        .children()
        .filter(|n| {
            n.tag_name().namespace() == Some("http://schemas.xmlsoap.org/ws/2004/08/addressing")
        })
        .collect::<Vec<_>>();

    println!("WSA Node: {:#?}", wsa_node);

    wsa_node.iter().for_each(|node| {
        println!("Node: {} =============", node.tag_name().name());
        node.children().for_each(|child| {
            if child.is_element() {
                println!("Element: {}", child.tag_name().name());
            }

            if child.is_text() {
                println!("Text: {}", child.text().unwrap_or("No text"));
            }
        });
        println!("Node: {} =============", node.tag_name().name());
    })

    // let itr = parsed
    //     .root()
    //     .first_child()
    //     .unwrap()
    //     .children()
    //     .filter(|n| {
    //         println!("Node: {:#?}", n.tag_name());
    //         n.tag_name().namespace() == Some("http://schemas.xmlsoap.org/ws/2004/08/addressing")
    //     });

    // for node in itr {
    //     println!(
    //         "Node: {} - Text: {}",
    //         node.tag_name().name(),
    //         node.text().unwrap_or("No text")
    //     );
    // }
}
