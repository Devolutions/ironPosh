use protocol::soap::header::SoapHeaders;
use xml::parser::XmlDeserialize;

const SOAP_HEADER: &'static str = r#"
    <s:Header>
    <a:To>
        http://10.10.0.3:5985/wsman?PSVersion=7.4.10
        </a:To>
    <w:ResourceURI
        s:mustUnderstand="true">
        http://schemas.microsoft.com/powershell/Microsoft.PowerShell
        </w:ResourceURI>
    <a:ReplyTo>
        <a:Address
            s:mustUnderstand="true">
            http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous
            </a:Address>
        </a:ReplyTo>
    <a:Action
        s:mustUnderstand="true">
        http://schemas.xmlsoap.org/ws/2004/09/transfer/Create
        </a:Action>
    <w:MaxEnvelopeSize
        s:mustUnderstand="true">
        512000
        </w:MaxEnvelopeSize>
    <a:MessageID>
        uuid:D1D65143-B634-4725-BBF6-869CC4D3062F
        </a:MessageID>
    <w:Locale
        xml:lang="en-US"
        s:mustUnderstand="false"/>
    <p:DataLocale
        xml:lang="en-CA"
        s:mustUnderstand="false"/>
    <p:SessionId
        s:mustUnderstand="false">
        uuid:9EC885D6-F5A4-4771-9D47-4BDF7DAAEA8C
        </p:SessionId>
    <p:OperationID
        s:mustUnderstand="false">
        uuid:73C4BCA6-7FF0-4AFE-B8C3-335FB19BA649
        </p:OperationID>
    <p:SequenceId
        s:mustUnderstand="false">
        1
        </p:SequenceId>
    <w:OptionSet
        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
        s:mustUnderstand="true">
        <w:Option
            Name="protocolversion"
            MustComply="true">
            2.3
            </w:Option>
        </w:OptionSet>
    <w:OperationTimeout>
        PT180.000S
        </w:OperationTimeout>
    <rsp:CompressionType
        s:mustUnderstand="true"
        xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
        xpress
        </rsp:CompressionType>
    </s:Header>
    "#;

pub fn main() {
    let node = xml::parser::parse(SOAP_HEADER).unwrap();
    let headers = SoapHeaders::from_node(node.root_element()).unwrap();
    assert!(headers.to.is_some());
    assert!(headers.resource_uri.is_some());
    assert!(headers.reply_to.is_some());
    assert!(headers.action.is_some());
    assert!(headers.max_envelope_size.is_some());
    assert!(headers.message_id.is_some());
    assert!(headers.locale.is_some());
    assert!(headers.data_locale.is_some());
    assert!(headers.session_id.is_some());
    assert!(headers.operation_id.is_some());
    assert!(headers.sequence_id.is_some());
    assert!(headers.option_set.is_some());
    assert!(headers.operation_timeout.is_some());
    assert!(headers.compression_type.is_some());

    // Additional assertions can be added to check the values of the fields
}
