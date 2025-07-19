use crate::{
    cores::{Tag, tag_name::*, tag_value::Text},
    push_elements,
};
use tracing::{debug, warn};
use xml::builder::Element;
use xml::parser::{XmlDeserialize, XmlVisitor};

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ShellValue<'a> {
    #[builder(default, setter(strip_option, into))]
    pub shell_id: Option<Tag<'a, Text<'a>, ShellId>>,
    #[builder(default, setter(strip_option, into))]
    pub name: Option<Tag<'a, Text<'a>, Name>>,
    #[builder(default, setter(strip_option, into))]
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceUri>>,
    #[builder(default, setter(strip_option, into))]
    pub owner: Option<Tag<'a, Text<'a>, Owner>>,
    #[builder(default, setter(strip_option, into))]
    pub client_ip: Option<Tag<'a, Text<'a>, ClientIP>>,
    #[builder(default, setter(strip_option, into))]
    pub process_id: Option<Tag<'a, Text<'a>, ProcessId>>,
    #[builder(default, setter(strip_option, into))]
    pub idle_time_out: Option<Tag<'a, Text<'a>, IdleTimeOut>>,
    #[builder(default, setter(strip_option, into))]
    pub input_streams: Option<Tag<'a, Text<'a>, InputStreams>>,
    #[builder(default, setter(strip_option, into))]
    pub output_streams: Option<Tag<'a, Text<'a>, OutputStreams>>,
    #[builder(default, setter(strip_option, into))]
    pub max_idle_time_out: Option<Tag<'a, Text<'a>, MaxIdleTimeOut>>,
    #[builder(default, setter(strip_option, into))]
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    #[builder(default, setter(strip_option, into))]
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    #[builder(default, setter(strip_option, into))]
    pub compression_mode: Option<Tag<'a, Text<'a>, CompressionMode>>,
    #[builder(default, setter(strip_option, into))]
    pub profile_loaded: Option<Tag<'a, Text<'a>, ProfileLoaded>>,
    #[builder(default, setter(strip_option, into))]
    pub encoding: Option<Tag<'a, Text<'a>, Encoding>>,
    #[builder(default, setter(strip_option, into))]
    pub buffer_mode: Option<Tag<'a, Text<'a>, BufferMode>>,
    #[builder(default, setter(strip_option, into))]
    pub state: Option<Tag<'a, Text<'a>, State>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_run_time: Option<Tag<'a, Text<'a>, ShellRunTime>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_inactivity: Option<Tag<'a, Text<'a>, ShellInactivity>>,
    #[builder(default, setter(strip_option, into))]
    pub creation_xml: Option<Tag<'a, Text<'a>, CreationXml>>,
}

impl<'a> crate::cores::TagValue<'a> for ShellValue<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        let Self {
            shell_id,
            name,
            resource_uri,
            owner,
            client_ip,
            process_id,
            idle_time_out,
            input_streams,
            output_streams,
            max_idle_time_out,
            locale,
            data_locale,
            compression_mode,
            profile_loaded,
            encoding,
            buffer_mode,
            state,
            shell_run_time,
            shell_inactivity,
            creation_xml,
        } = self;

        let mut array = Vec::new();

        // Add optional elements
        push_elements!(
            array,
            [
                shell_id,
                name,
                resource_uri,
                owner,
                client_ip,
                process_id,
                idle_time_out,
                input_streams,
                output_streams,
                max_idle_time_out,
                locale,
                data_locale,
                compression_mode,
                profile_loaded,
                encoding,
                buffer_mode,
                state,
                shell_run_time,
                shell_inactivity,
                creation_xml
            ]
        );

        element.add_children(array)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ShellValueVisitor<'a> {
    pub shell_id: Option<Tag<'a, Text<'a>, ShellId>>,
    pub name: Option<Tag<'a, Text<'a>, Name>>,
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceUri>>,
    pub owner: Option<Tag<'a, Text<'a>, Owner>>,
    pub client_ip: Option<Tag<'a, Text<'a>, ClientIP>>,
    pub process_id: Option<Tag<'a, Text<'a>, ProcessId>>,
    pub idle_time_out: Option<Tag<'a, Text<'a>, IdleTimeOut>>,
    pub input_streams: Option<Tag<'a, Text<'a>, InputStreams>>,
    pub output_streams: Option<Tag<'a, Text<'a>, OutputStreams>>,
    pub max_idle_time_out: Option<Tag<'a, Text<'a>, MaxIdleTimeOut>>,
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    pub compression_mode: Option<Tag<'a, Text<'a>, CompressionMode>>,
    pub profile_loaded: Option<Tag<'a, Text<'a>, ProfileLoaded>>,
    pub encoding: Option<Tag<'a, Text<'a>, Encoding>>,
    pub buffer_mode: Option<Tag<'a, Text<'a>, BufferMode>>,
    pub state: Option<Tag<'a, Text<'a>, State>>,
    pub shell_run_time: Option<Tag<'a, Text<'a>, ShellRunTime>>,
    pub shell_inactivity: Option<Tag<'a, Text<'a>, ShellInactivity>>,
    pub creation_xml: Option<Tag<'a, Text<'a>, CreationXml>>,
}

impl<'a> XmlVisitor<'a> for ShellValueVisitor<'a> {
    type Value = ShellValue<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        for child in children {
            if !child.is_element() {
                continue; // Skip non-element nodes like text/whitespace
            }

            let tag_name = child.tag_name().name();
            let namespace = child.tag_name().namespace();

            debug!(
                "Processing child element: tag_name='{}', namespace={:?}",
                tag_name, namespace
            );

            match tag_name {
                ShellId::TAG_NAME => {
                    debug!("Found ShellId element");
                    self.shell_id = Some(Tag::from_node(child)?);
                }
                Name::TAG_NAME => {
                    debug!("Found Name element");
                    self.name = Some(Tag::from_node(child)?);
                }
                ResourceUri::TAG_NAME => {
                    debug!("Found ResourceUri element");
                    self.resource_uri = Some(Tag::from_node(child)?);
                }
                Owner::TAG_NAME => {
                    debug!("Found Owner element");
                    self.owner = Some(Tag::from_node(child)?);
                }
                ClientIP::TAG_NAME => {
                    debug!("Found ClientIP element");
                    self.client_ip = Some(Tag::from_node(child)?);
                }
                ProcessId::TAG_NAME => {
                    debug!("Found ProcessId element");
                    self.process_id = Some(Tag::from_node(child)?);
                }
                IdleTimeOut::TAG_NAME => {
                    debug!("Found IdleTimeOut element");
                    self.idle_time_out = Some(Tag::from_node(child)?);
                }
                InputStreams::TAG_NAME => {
                    debug!("Found InputStreams element");
                    self.input_streams = Some(Tag::from_node(child)?);
                }
                OutputStreams::TAG_NAME => {
                    debug!("Found OutputStreams element");
                    self.output_streams = Some(Tag::from_node(child)?);
                }
                MaxIdleTimeOut::TAG_NAME => {
                    debug!("Found MaxIdleTimeOut element");
                    self.max_idle_time_out = Some(Tag::from_node(child)?);
                }
                Locale::TAG_NAME => {
                    debug!("Found Locale element");
                    self.locale = Some(Tag::from_node(child)?);
                }
                DataLocale::TAG_NAME => {
                    debug!("Found DataLocale element");
                    self.data_locale = Some(Tag::from_node(child)?);
                }
                CompressionMode::TAG_NAME => {
                    debug!("Found CompressionMode element");
                    self.compression_mode = Some(Tag::from_node(child)?);
                }
                ProfileLoaded::TAG_NAME => {
                    debug!("Found ProfileLoaded element");
                    self.profile_loaded = Some(Tag::from_node(child)?);
                }
                Encoding::TAG_NAME => {
                    debug!("Found Encoding element");
                    self.encoding = Some(Tag::from_node(child)?);
                }
                BufferMode::TAG_NAME => {
                    debug!("Found BufferMode element");
                    self.buffer_mode = Some(Tag::from_node(child)?);
                }
                State::TAG_NAME => {
                    debug!("Found State element");
                    self.state = Some(Tag::from_node(child)?);
                }
                ShellRunTime::TAG_NAME => {
                    debug!("Found ShellRunTime element");
                    self.shell_run_time = Some(Tag::from_node(child)?);
                }
                ShellInactivity::TAG_NAME => {
                    debug!("Found ShellInactivity element");
                    self.shell_inactivity = Some(Tag::from_node(child)?);
                }
                CreationXml::TAG_NAME => {
                    debug!("Found CreationXml element");
                    self.creation_xml = Some(Tag::from_node(child)?);
                }
                _ => {
                    warn!(
                        "Unknown tag in Shell: '{}' (namespace: {:?})",
                        tag_name, namespace
                    );
                    return Err(xml::XmlError::InvalidXml(format!(
                        "Unknown tag in Shell: {tag_name}"
                    )));
                }
            }
        }

        Ok(())
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        debug!("ShellVisitor visiting node: {:?}", node.tag_name());

        // Get the children and process them
        let children: Vec<_> = node.children().collect();
        debug!("Found {} children", children.len());

        self.visit_children(children.into_iter())?;
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        let Self {
            shell_id,
            name,
            resource_uri,
            owner,
            client_ip,
            process_id,
            idle_time_out,
            input_streams,
            output_streams,
            max_idle_time_out,
            locale,
            data_locale,
            compression_mode,
            profile_loaded,
            encoding,
            buffer_mode,
            state,
            shell_run_time,
            shell_inactivity,
            creation_xml,
        } = self;

        Ok(ShellValue {
            shell_id,
            name,
            resource_uri,
            owner,
            client_ip,
            process_id,
            idle_time_out,
            input_streams,
            output_streams,
            max_idle_time_out,
            locale,
            data_locale,
            compression_mode,
            profile_loaded,
            encoding,
            buffer_mode,
            state,
            shell_run_time,
            shell_inactivity,
            creation_xml,
        })
    }
}

impl<'a> XmlDeserialize<'a> for ShellValue<'a> {
    type Visitor = ShellValueVisitor<'a>;

    fn visitor() -> Self::Visitor {
        ShellValueVisitor::default()
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
