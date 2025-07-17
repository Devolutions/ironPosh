use tracing::{debug, warn};
use xml::parser::{XmlDeserialize, XmlVisitor};

use crate::{cores::*, push_elements};

#[derive(Debug, Clone)]
pub struct SoapBody<'a> {
    /// WS-Management operations
    pub identify: Option<Tag<'a, Empty, Identify>>,
    pub get: Option<Tag<'a, Text<'a>, Get>>,
    pub put: Option<Tag<'a, Text<'a>, Put>>,
    pub create: Option<Tag<'a, Text<'a>, Create>>,
    pub delete: Option<Tag<'a, Text<'a>, Delete>>,
    pub enumerate: Option<Tag<'a, TagList<'a>, Enumerate>>,
    pub pull: Option<Tag<'a, TagList<'a>, Pull>>,
    pub release: Option<Tag<'a, TagList<'a>, Release>>,
    pub get_status: Option<Tag<'a, TagList<'a>, GetStatus>>,

    /// PowerShell Remoting operations
    pub shell: Option<Tag<'a, TagList<'a>, Shell>>,
    pub command: Option<Tag<'a, TagList<'a>, Command>>,
    pub receive: Option<Tag<'a, TagList<'a>, Receive>>,
    pub send: Option<Tag<'a, TagList<'a>, Send>>,
    pub signal: Option<Tag<'a, TagList<'a>, Signal>>,

    /// Custom/Generic content
    pub custom_content: Option<TagList<'a>>,
}

impl<'a> TagValue<'a> for SoapBody<'a> {
    fn into_element(
        self,
        name: &'static str,
        namespace: Option<&'static str>,
    ) -> xml::builder::Element<'a> {
        let mut body = xml::builder::Element::new(name).set_namespace_optional(namespace);

        let mut array = Vec::new();

        let Self {
            identify,
            get,
            put,
            create,
            delete,
            enumerate,
            pull,
            release,
            get_status,
            shell,
            command,
            receive,
            send,
            signal,
            custom_content,
        } = self;

        push_elements!(
            array,
            [
                identify, get, put, create, delete, enumerate, pull, release, get_status, shell,
                command, receive, send, signal
            ]
        );

        // Add custom content if present
        if let Some(content) = custom_content {
            body = body.add_child(content.into_element("CustomContent", None));
        }

        body = body.add_children(array);
        body
    }
}

#[derive(Debug, Clone, Default)]
pub struct SoapBodyVisitor<'a> {
    /// WS-Management operations
    pub identify: Option<Tag<'a, Empty, Identify>>,
    pub get: Option<Tag<'a, Text<'a>, Get>>,
    pub put: Option<Tag<'a, Text<'a>, Put>>,
    pub create: Option<Tag<'a, Text<'a>, Create>>,
    pub delete: Option<Tag<'a, Text<'a>, Delete>>,
    pub enumerate: Option<Tag<'a, TagList<'a>, Enumerate>>,
    pub pull: Option<Tag<'a, TagList<'a>, Pull>>,
    pub release: Option<Tag<'a, TagList<'a>, Release>>,
    pub get_status: Option<Tag<'a, TagList<'a>, GetStatus>>,

    /// PowerShell Remoting operations
    pub shell: Option<Tag<'a, TagList<'a>, Shell>>,
    pub command: Option<Tag<'a, TagList<'a>, Command>>,
    pub receive: Option<Tag<'a, TagList<'a>, Receive>>,
    pub send: Option<Tag<'a, TagList<'a>, Send>>,
    pub signal: Option<Tag<'a, TagList<'a>, Signal>>,

    /// Custom/Generic content
    pub custom_content: Option<TagList<'a>>,
}

impl<'a> XmlVisitor<'a> for SoapBodyVisitor<'a> {
    type Value = SoapBody<'a>;

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

            debug!(
                "Processing child element: tag_name='{}', namespace={:?}",
                tag_name, namespace
            );

            match tag_name {
                // WS-Management operations
                Identify::TAG_NAME => {
                    debug!("Found Identify element");
                    self.identify = Some(Tag::from_node(node)?);
                }
                Get::TAG_NAME => {
                    debug!("Found Get element");
                    self.get = Some(Tag::from_node(node)?);
                }
                Put::TAG_NAME => {
                    debug!("Found Put element");
                    self.put = Some(Tag::from_node(node)?);
                }
                Create::TAG_NAME => {
                    debug!("Found Create element");
                    self.create = Some(Tag::from_node(node)?);
                }
                Delete::TAG_NAME => {
                    debug!("Found Delete element");
                    self.delete = Some(Tag::from_node(node)?);
                }
                Enumerate::TAG_NAME => {
                    debug!("Found Enumerate element");
                    self.enumerate = Some(Tag::from_node(node)?);
                }
                Pull::TAG_NAME => {
                    debug!("Found Pull element");
                    self.pull = Some(Tag::from_node(node)?);
                }
                Release::TAG_NAME => {
                    debug!("Found Release element");
                    self.release = Some(Tag::from_node(node)?);
                }
                GetStatus::TAG_NAME => {
                    debug!("Found GetStatus element");
                    self.get_status = Some(Tag::from_node(node)?);
                }
                // PowerShell Remoting operations
                Shell::TAG_NAME => {
                    debug!("Found Shell element");
                    self.shell = Some(Tag::from_node(node)?);
                }
                Command::TAG_NAME => {
                    debug!("Found Command element");
                    self.command = Some(Tag::from_node(node)?);
                }
                Receive::TAG_NAME => {
                    debug!("Found Receive element");
                    self.receive = Some(Tag::from_node(node)?);
                }
                Send::TAG_NAME => {
                    debug!("Found Send element");
                    self.send = Some(Tag::from_node(node)?);
                }
                Signal::TAG_NAME => {
                    debug!("Found Signal element");
                    self.signal = Some(Tag::from_node(node)?);
                }
                _ => {
                    warn!(
                        "Unknown or custom tag in SOAP body: '{}' (namespace: {:?})",
                        tag_name, namespace
                    );
                    // Instead of erroring, we'll collect custom content
                    if self.custom_content.is_none() {
                        self.custom_content = Some(TagList::visitor().finish()?);
                    }
                    // Note: In a real implementation, you'd want to properly handle custom content
                    // For now, we'll just log and continue
                }
            }
        }

        Ok(())
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        debug!("SoapBodyVisitor visiting node: {:?}", node.tag_name());

        // Get the children and process them
        let children: Vec<_> = node.children().collect();
        debug!("Found {} children", children.len());

        self.visit_children(children.into_iter())?;
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        let Self {
            identify,
            get,
            put,
            create,
            delete,
            enumerate,
            pull,
            release,
            get_status,
            shell,
            command,
            receive,
            send,
            signal,
            custom_content,
        } = self;

        Ok(SoapBody {
            identify,
            get,
            put,
            create,
            delete,
            enumerate,
            pull,
            release,
            get_status,
            shell,
            command,
            receive,
            send,
            signal,
            custom_content,
        })
    }
}

impl<'a> XmlDeserialize<'a> for SoapBody<'a> {
    type Visitor = SoapBodyVisitor<'a>;

    fn visitor() -> Self::Visitor {
        SoapBodyVisitor::default()
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
