use xml::parser::{XmlDeserialize, XmlVisitor};

use crate::{push_elements, traits::*};

#[derive(Debug, Clone)]
pub struct SoapHeaders<'a> {
    /// WS-Addressing headers
    pub to: Option<Tag<'a, Text<'a>, To>>,
    pub action: Option<Tag<'a, Text<'a>, Action>>,
    pub reply_to: Option<Tag<'a, TagList<'a>, ReplyTo>>,
    pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,

    /// WS-Management headers
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceURI>>,
    pub max_envelope_size: Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    pub session_id: Option<Tag<'a, Text<'a>, SessionId>>,
    pub operation_id: Option<Tag<'a, Text<'a>, OperationID>>,
    pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
    pub option_set: Option<Tag<'a, TagList<'a>, OptionSet>>,
    pub operation_timeout: Option<Tag<'a, Text<'a>, OperationTimeout>>,
    pub compression_type: Option<Tag<'a, TagList<'a>, CompressionType>>,
}

impl<'a> TagValue<'a> for SoapHeaders<'a> {
    fn into_element(
        self,
        name: &'static str,
        namespace: Option<&'static str>,
    ) -> xml::builder::Element<'a> {
        let mut header = xml::builder::Element::new(name).set_namespace_optional(namespace);

        let mut array = Vec::new();

        let Self {
            to,
            action,
            reply_to,
            message_id,
            resource_uri,
            max_envelope_size,
            locale,
            data_locale,
            session_id,
            operation_id,
            sequence_id,
            option_set,
            operation_timeout,
            compression_type,
        } = self;

        push_elements!(
            array,
            [
                to,
                action,
                reply_to,
                message_id,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type
            ]
        );

        header = header.add_children(array);
        header
    }
}

#[derive(Debug, Clone, Default)]
pub struct SoapHeaderVisitor<'a> {
    /// WS-Addressing headers
    pub to: Option<Tag<'a, Text<'a>, To>>,
    pub action: Option<Tag<'a, Text<'a>, Action>>,
    pub reply_to: Option<Tag<'a, TagList<'a>, ReplyTo>>,
    pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,

    /// WS-Management headers
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceURI>>,
    pub max_envelope_size: Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    pub session_id: Option<Tag<'a, Text<'a>, SessionId>>,
    pub operation_id: Option<Tag<'a, Text<'a>, OperationID>>,
    pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
    pub option_set: Option<Tag<'a, TagList<'a>, OptionSet>>,
    pub operation_timeout: Option<Tag<'a, Text<'a>, OperationTimeout>>,
    pub compression_type: Option<Tag<'a, TagList<'a>, CompressionType>>,
}

impl<'a> XmlVisitor<'a> for SoapHeaderVisitor<'a> {
    type Value = SoapHeaders<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        for node in children {
            match node.tag_name().name() {
                To::TAG_NAME => {
                    self.to = Some(Tag::from_node(node)?);
                }
                Action::TAG_NAME => {
                    self.action = Some(Tag::from_node(node)?);
                }
                ReplyTo::TAG_NAME => {
                    self.reply_to = Some(Tag::from_node(node)?);
                }
                MessageID::TAG_NAME => {
                    self.message_id = Some(Tag::from_node(node)?);
                }
                ResourceURI::TAG_NAME => {
                    self.resource_uri = Some(Tag::from_node(node)?);
                }
                MaxEnvelopeSize::TAG_NAME => {
                    self.max_envelope_size = Some(Tag::from_node(node)?);
                }
                Locale::TAG_NAME => {
                    self.locale = Some(Tag::from_node(node)?);
                }
                DataLocale::TAG_NAME => {
                    self.data_locale = Some(Tag::from_node(node)?);
                }
                SessionId::TAG_NAME => {
                    self.session_id = Some(Tag::from_node(node)?);
                }
                OperationID::TAG_NAME => {
                    self.operation_id = Some(Tag::from_node(node)?);
                }
                SequenceId::TAG_NAME => {
                    self.sequence_id = Some(Tag::from_node(node)?);
                }
                OptionSet::TAG_NAME => {
                    self.option_set = Some(Tag::from_node(node)?);
                }
                OperationTimeout::TAG_NAME => {
                    self.operation_timeout = Some(Tag::from_node(node)?);
                }
                CompressionType::TAG_NAME => {
                    self.compression_type = Some(Tag::from_node(node)?);
                }
                _ => {
                    return Err(xml::XmlError::InvalidXml(format!(
                        "Unknown tag in SOAP header: {}",
                        node.tag_name().name()
                    )));
                }
            }
        }

        Ok(())
    }

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        let Self {
            to,
            action,
            reply_to,
            message_id,
            resource_uri,
            max_envelope_size,
            locale,
            data_locale,
            session_id,
            operation_id,
            sequence_id,
            option_set,
            operation_timeout,
            compression_type,
        } = self;

        Ok(SoapHeaders {
            to,
            action,
            reply_to,
            message_id,
            resource_uri,
            max_envelope_size,
            locale,
            data_locale,
            session_id,
            operation_id,
            sequence_id,
            option_set,
            operation_timeout,
            compression_type,
        })
    }
}

impl<'a> XmlDeserialize<'a> for SoapHeaders<'a> {
    type Visitor = SoapHeaderVisitor<'a>;

    fn visitor() -> Self::Visitor {
        SoapHeaderVisitor::default()
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

#[cfg(test)]
mod test {

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

    use super::*;

    #[test]
    fn test_soap_header_deserialization() {
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
}
