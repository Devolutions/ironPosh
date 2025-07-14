use xml::parser::Node;

use crate::{
    must_be_element, must_be_tag, must_have_namespace,
    soap::{SOAP_NAMESPACE, SoapHeaders},
    ws_addressing::WsAddressingHeaders,
    ws_management::WsManagementHeader,
};

pub struct Soap<'a> {
    pub ws_addressing_header: Option<crate::ws_addressing::WsAddressingHeaders<'a>>,
    pub ws_management_header: Option<crate::ws_management::WsManagementHeader<'a>>,

    __phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> TryFrom<Node<'a, 'a>> for Soap<'a> {
    type Error = xml::XmlError<'a>;

    fn try_from(node: Node<'a, 'a>) -> Result<Self, Self::Error> {
        must_be_element!(&node);
        must_be_tag!(&node, "Envelope");
        must_have_namespace!(&node, SOAP_NAMESPACE);

        let mut ws_addressing_header = None;
        let mut ws_management_header = None;

        let header_tag = node.children().find(|child| {
            child.is_element()
                && child.tag_name().namespace() == Some(SOAP_NAMESPACE)
                && child.tag_name().name() == "Header"
        });

        // debug purpose only
        #[cfg(test)]
        {
            header_tag.map(|header_tag| {
                header_tag.children().for_each(|child| {
                    if child.is_element() {
                        println!("Processing child: {:?}", child.tag_name());
                        println!(
                            "Namespace: {:?}, == {:?}",
                            child.tag_name().namespace(),
                            child.tag_name().namespace() == Some(WsAddressingHeaders::NAMESPACE)
                        );
                    }
                });
            });
        }

        if let Some(header_tag) = header_tag {
            let ws_addr_headers_nodes: Vec<_> = header_tag
                .children()
                .filter(|children| {
                    children.is_element()
                        && children.tag_name().namespace() == Some(WsAddressingHeaders::NAMESPACE)
                })
                .collect();

            let ws_man_headers_nodes: Vec<_> = header_tag
                .children()
                .filter(|children| {
                    children.is_element()
                        && children.tag_name().namespace() == Some(WsManagementHeader::NAMESPACE)
                })
                .collect();

            ws_addressing_header = Some(WsAddressingHeaders::try_from(ws_addr_headers_nodes)?);
            ws_management_header = Some(WsManagementHeader::try_from(ws_man_headers_nodes)?);
        };

        Ok(Soap {
            ws_addressing_header,
            ws_management_header,
            __phantom: std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use xml::parser::parse;

    #[test]
    fn test_soap_parsing() {
        let soap_xml = r#"
        <s:Envelope
    xml:lang="en-US"
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:x="http://schemas.xmlsoap.org/ws/2004/09/transfer"
    xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
    xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell"
    xmlns:p="http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd">
    <s:Header>
        <a:Action>
            http://schemas.xmlsoap.org/ws/2004/09/transfer/CreateResponse
            </a:Action>
        <a:MessageID>
            uuid:E17CCBB8-6136-4FA1-95B2-0DEF618A9232
            </a:MessageID>
        <p:OperationID
            s:mustUnderstand="false">
            uuid:73C4BCA6-7FF0-4AFE-B8C3-335FB19BA649
            </p:OperationID>
        <p:SequenceId>
            1
            </p:SequenceId>
        <a:To>
            http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous
            </a:To>
        <a:RelatesTo>
            uuid:D1D65143-B634-4725-BBF6-869CC4D3062F
            </a:RelatesTo>
        </s:Header>
    <s:Body>
        <x:ResourceCreated>
            <a:Address>
                http://10.10.0.3:5985/wsman?PSVersion=7.4.10
                </a:Address>
            <a:ReferenceParameters>
                <w:ResourceURI>
                    http://schemas.microsoft.com/powershell/Microsoft.PowerShell
                    </w:ResourceURI>
                <w:SelectorSet>
                    <w:Selector
                        Name="ShellId">
                        2D6534D0-6B12-40E3-B773-CBA26459CFA8
                        </w:Selector>
                    </w:SelectorSet>
                </a:ReferenceParameters>
            </x:ResourceCreated>
        <rsp:Shell
            xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
            <rsp:ShellId>
                2D6534D0-6B12-40E3-B773-CBA26459CFA8
                </rsp:ShellId>
            <rsp:Name>
                Runspace1
                </rsp:Name>
            <rsp:ResourceUri>
                http://schemas.microsoft.com/powershell/Microsoft.PowerShell
                </rsp:ResourceUri>
            <rsp:Owner>
                administrator
                </rsp:Owner>
            <rsp:ClientIP>
                10.10.0.1
                </rsp:ClientIP>
            <rsp:ProcessId>
                5812
                </rsp:ProcessId>
            <rsp:IdleTimeOut>
                PT7200.000S
                </rsp:IdleTimeOut>
            <rsp:InputStreams>
                stdin pr
                </rsp:InputStreams>
            <rsp:OutputStreams>
                stdout
                </rsp:OutputStreams>
            <rsp:MaxIdleTimeOut>
                PT2147483.647S
                </rsp:MaxIdleTimeOut>
            <rsp:Locale>
                en-US
                </rsp:Locale>
            <rsp:DataLocale>
                en-CA
                </rsp:DataLocale>
            <rsp:CompressionMode>
                XpressCompression
                </rsp:CompressionMode>
            <rsp:ProfileLoaded>
                Yes
                </rsp:ProfileLoaded>
            <rsp:Encoding>
                UTF8
                </rsp:Encoding>
            <rsp:BufferMode>
                Block
                </rsp:BufferMode>
            <rsp:State>
                Connected
                </rsp:State>
            <rsp:ShellRunTime>
                P0DT0H0M0S
                </rsp:ShellRunTime>
            <rsp:ShellInactivity>
                P0DT0H0M0S
                </rsp:ShellInactivity>
            </rsp:Shell>
        </s:Body>
    </s:Envelope>
"#;

        let parsed = parse(soap_xml).expect("Failed to parse SOAP XML");
        let root = parsed.root();
        let envelope = root.first_child().expect("should have first child");

        let soap = Soap::try_from(envelope).expect("failed to parse soap");

        assert!(soap.ws_addressing_header.is_some());
        assert!(soap.ws_management_header.is_some());

        let addressing_header = soap.ws_addressing_header.unwrap();
        assert_eq!(
            addressing_header.to.unwrap().value,
            "http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous"
        );
    }
}
