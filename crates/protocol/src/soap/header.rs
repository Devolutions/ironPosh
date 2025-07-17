use xml::parser::{XmlDeserialize, XmlVisitor};
use tracing::{debug, warn};

use crate::{push_elements, cores::*};

#[derive(Debug, Clone)]
pub struct SoapHeaders<'a> {
    /// WS-Addressing headers
    pub to: Option<Tag<'a, Text<'a>, To>>,
    pub action: Option<Tag<'a, Text<'a>, Action>>,
    pub reply_to: Option<Tag<'a, TagList<'a>, ReplyTo>>,
    pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,
    pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,

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
            relates_to,
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
                relates_to,
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
    pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,

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
            if !node.is_element() {
                continue; // Skip non-element nodes like text/whitespace
            }
            
            let tag_name = node.tag_name().name();
            let namespace = node.tag_name().namespace();
            
            debug!("Processing child element: tag_name='{}', namespace={:?}", tag_name, namespace);
            
            match tag_name {
                To::TAG_NAME => {
                    debug!("Found To element");
                    self.to = Some(Tag::from_node(node)?);
                }
                Action::TAG_NAME => {
                    debug!("Found Action element");
                    self.action = Some(Tag::from_node(node)?);
                }
                ReplyTo::TAG_NAME => {
                    debug!("Found ReplyTo element");
                    self.reply_to = Some(Tag::from_node(node)?);
                }
                MessageID::TAG_NAME => {
                    debug!("Found MessageID element");
                    self.message_id = Some(Tag::from_node(node)?);
                }
                RelatesTo::TAG_NAME => {
                    debug!("Found RelatesTo element");
                    self.relates_to = Some(Tag::from_node(node)?);
                }
                ResourceURI::TAG_NAME => {
                    debug!("Found ResourceURI element");
                    self.resource_uri = Some(Tag::from_node(node)?);
                }
                MaxEnvelopeSize::TAG_NAME => {
                    debug!("Found MaxEnvelopeSize element");
                    self.max_envelope_size = Some(Tag::from_node(node)?);
                }
                Locale::TAG_NAME => {
                    debug!("Found Locale element");
                    self.locale = Some(Tag::from_node(node)?);
                }
                DataLocale::TAG_NAME => {
                    debug!("Found DataLocale element");
                    self.data_locale = Some(Tag::from_node(node)?);
                }
                SessionId::TAG_NAME => {
                    debug!("Found SessionId element");
                    self.session_id = Some(Tag::from_node(node)?);
                }
                OperationID::TAG_NAME => {
                    debug!("Found OperationID element");
                    self.operation_id = Some(Tag::from_node(node)?);
                }
                SequenceId::TAG_NAME => {
                    debug!("Found SequenceId element");
                    self.sequence_id = Some(Tag::from_node(node)?);
                }
                OptionSet::TAG_NAME => {
                    debug!("Found OptionSet element");
                    self.option_set = Some(Tag::from_node(node)?);
                }
                OperationTimeout::TAG_NAME => {
                    debug!("Found OperationTimeout element");
                    self.operation_timeout = Some(Tag::from_node(node)?);
                }
                CompressionType::TAG_NAME => {
                    debug!("Found CompressionType element");
                    self.compression_type = Some(Tag::from_node(node)?);
                }
                _ => {
                    warn!("Unknown tag in SOAP header: '{}' (namespace: {:?})", tag_name, namespace);
                    return Err(xml::XmlError::InvalidXml(format!(
                        "Unknown tag in SOAP header: {}",
                        tag_name
                    )));
                }
            }
        }

        Ok(())
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        debug!("SoapHeaderVisitor visiting node: {:?}", node.tag_name());
        
        // Get the children and process them
        let children: Vec<_> = node.children().collect();
        debug!("Found {} children", children.len());
        
        self.visit_children(children.into_iter())?;
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        let Self {
            to,
            action,
            reply_to,
            message_id,
            relates_to,
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
            relates_to,
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
