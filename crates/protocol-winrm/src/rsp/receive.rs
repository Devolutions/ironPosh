use protocol_macros::{SimpleTagValue, SimpleXmlDeserialize};

use crate::cores::{
    DesiredStream, Tag, TagName, TagValue, Text,
    attribute::{self, Attribute},
};
use std::borrow::Cow;
use xml::{
    XmlError,
    builder::Element,
    parser::{self, Error, Node, XmlDeserialize, XmlVisitor},
};

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct ReceiveValue<'a> {
    pub desired_stream: Tag<'a, Text<'a>, DesiredStream>,
}

// Stream element for ReceiveResponse
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct StreamValue<'a> {
    #[builder(setter(into))]
    pub name: Cow<'a, str>, // Name attribute
    #[builder(default, setter(into, strip_option))]
    pub command_id: Option<Cow<'a, str>>, // CommandId attribute
    #[builder(default, setter(into, strip_option))]
    pub end: Option<bool>, // End attribute
    #[builder(default, setter(into, strip_option))]
    pub unit: Option<Cow<'a, str>>, // Unit attribute
    #[builder(default, setter(into, strip_option))]
    pub end_unit: Option<bool>, // EndUnit attribute
    #[builder(setter(into))]
    pub content: Text<'a>, // base64-encoded stream data
}

// CommandState element for ReceiveResponse
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct CommandStateValue<'a> {
    #[builder(setter(into))]
    pub command_id: Cow<'a, str>, // CommandId attribute
    #[builder(setter(into))]
    pub state: Cow<'a, str>, // State attribute
    #[builder(default, setter(into, strip_option))]
    pub exit_code: Option<Tag<'a, Text<'a>, crate::cores::tag_name::ExitCode>>,
}

// ReceiveResponse main structure
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ReceiveResponseValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub sequence_id: Option<u64>, // SequenceID attribute
    #[builder(setter(into))]
    pub streams: Vec<Tag<'a, StreamValue<'a>, crate::cores::tag_name::Stream>>,
    #[builder(default, setter(into, strip_option))]
    pub command_state: Option<Tag<'a, CommandStateValue<'a>, crate::cores::tag_name::CommandState>>,
}

// TagValue implementations
impl<'a> TagValue<'a> for StreamValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        // Add required Name attribute
        element = element.add_attribute(attribute::Attribute::Name(self.name).into());

        // Add optional attributes
        if let Some(command_id) = self.command_id {
            element = element.add_attribute(attribute::Attribute::CommandId(command_id).into());
        }
        if let Some(end) = self.end {
            element = element.add_attribute(attribute::Attribute::End(end).into());
        }
        if let Some(unit) = self.unit {
            element = element.add_attribute(attribute::Attribute::Unit(unit).into());
        }
        if let Some(end_unit) = self.end_unit {
            element = element.add_attribute(attribute::Attribute::EndUnit(end_unit).into());
        }

        // Set content (base64-encoded stream data)
        element.set_text(self.content)
    }
}

impl<'a> TagValue<'a> for CommandStateValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        // Add required attributes
        element = element.add_attribute(attribute::Attribute::CommandId(self.command_id).into());
        element = element.add_attribute(attribute::Attribute::State(self.state).into());

        // Add optional ExitCode child element
        if let Some(exit_code) = self.exit_code {
            element = exit_code.append_to_element(element);
        }

        element
    }
}

impl<'a> TagValue<'a> for ReceiveResponseValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        // Add optional SequenceID attribute
        if let Some(sequence_id) = self.sequence_id {
            element = element.add_attribute(attribute::Attribute::SequenceID(sequence_id).into());
        }

        // Add Stream elements (at least one required)
        for stream in self.streams {
            element = stream.append_to_element(element);
        }

        // Add optional CommandState element
        if let Some(command_state) = self.command_state {
            element = command_state.append_to_element(element);
        }

        element
    }
}

// XmlDeserialize implementations using visitor pattern
pub struct StreamValueVisitor<'a> {
    name: Option<Cow<'a, str>>,
    command_id: Option<Cow<'a, str>>,
    end: Option<bool>,
    unit: Option<Cow<'a, str>>,
    end_unit: Option<bool>,
    content: Option<Text<'a>>,
}

impl<'a> XmlVisitor<'a> for StreamValueVisitor<'a> {
    type Value = StreamValue<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        todo!()
    }

    fn finish(self) -> Result<Self::Value, XmlError> {
        Ok(StreamValue {
            name: self
                .name
                .ok_or_else(|| XmlError::InvalidXml("Missing Name attribute".into()))?,
            command_id: self.command_id,
            end: self.end,
            unit: self.unit,
            end_unit: self.end_unit,
            content: self
                .content
                .ok_or_else(|| XmlError::InvalidXml("Missing stream content".into()))?,
        })
    }
}

impl<'a> XmlDeserialize<'a> for StreamValue<'a> {
    type Visitor = StreamValueVisitor<'a>;

    fn visitor() -> Self::Visitor {
        StreamValueVisitor {
            name: None,
            command_id: None,
            end: None,
            unit: None,
            end_unit: None,
            content: None,
        }
    }
}

pub struct CommandStateValueVisitor<'a> {
    command_id: Option<Cow<'a, str>>,
    state: Option<Cow<'a, str>>,
    exit_code: Option<Tag<'a, Text<'a>, crate::cores::tag_name::ExitCode>>,
}

impl<'a> XmlVisitor<'a> for CommandStateValueVisitor<'a> {
    type Value = CommandStateValue<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = Node<'a, 'a>>,
    ) -> Result<(), XmlError> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, XmlError> {
        Ok(CommandStateValue {
            command_id: self
                .command_id
                .ok_or_else(|| XmlError::InvalidXml("Missing CommandId attribute".into()))?,
            state: self
                .state
                .ok_or_else(|| XmlError::InvalidXml("Missing State attribute".into()))?,
            exit_code: self.exit_code,
        })
    }
}

impl<'a> XmlDeserialize<'a> for CommandStateValue<'a> {
    type Visitor = CommandStateValueVisitor<'a>;

    fn visitor() -> Self::Visitor {
        CommandStateValueVisitor {
            command_id: None,
            state: None,
            exit_code: None,
        }
    }

    fn from_children(children: impl Iterator<Item = Node<'a, 'a>>) -> Result<Self, XmlError> {
        let mut visitor = Self::visitor();

        for child in children {
            visitor.visit_node(child)?;
        }

        visitor.finish()
    }
}

pub struct ReceiveResponseValueVisitor<'a> {
    sequence_id: Option<u64>,
    streams: Vec<Tag<'a, StreamValue<'a>, crate::cores::tag_name::Stream>>,
    command_state: Option<Tag<'a, CommandStateValue<'a>, crate::cores::tag_name::CommandState>>,
}

impl<'a> XmlVisitor<'a> for ReceiveResponseValueVisitor<'a> {
    type Value = ReceiveResponseValue<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = Node<'a, 'a>>,
    ) -> Result<(), XmlError> {
        for child in children {
            match child.tag_name().name() {
                "Stream" => {
                    let stream_value = StreamValue::from_node(child)?;
                    self.streams
                        .push(Tag::new(stream_value, crate::cores::tag_name::Stream));
                }
                "CommandState" => {
                    let command_state_value = CommandStateValue::from_node(child)?;
                    self.command_state = Some(Tag::new(
                        command_state_value,
                        crate::cores::tag_name::CommandState,
                    ));
                }
                _ => {} // Ignore unknown elements
            }
        }
        Ok(())
    }

    fn visit_node(&mut self, node: Node<'a, 'a>) -> Result<(), XmlError> {
        // Parse attributes from the node
        for attr in node.attributes() {
            if let Some(attribute::Attribute::SequenceID(seq_id)) =
                attribute::Attribute::from_name_and_value(attr.name(), attr.value())?
            {
                self.sequence_id = Some(seq_id);
            }
        }

        // Visit children to parse Stream and CommandState elements
        self.visit_children(node.children().filter(|c| c.is_element()))?;

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, XmlError> {
        if self.streams.is_empty() {
            return Err(XmlError::InvalidXml(
                "ReceiveResponse must have at least one Stream element".into(),
            ));
        }

        Ok(ReceiveResponseValue {
            sequence_id: self.sequence_id,
            streams: self.streams,
            command_state: self.command_state,
        })
    }
}

impl<'a> XmlDeserialize<'a> for ReceiveResponseValue<'a> {
    type Visitor = ReceiveResponseValueVisitor<'a>;

    fn visitor() -> Self::Visitor {
        ReceiveResponseValueVisitor {
            sequence_id: None,
            streams: Vec::new(),
            command_state: None,
        }
    }

    fn from_children(children: impl Iterator<Item = Node<'a, 'a>>) -> Result<Self, XmlError> {
        let mut visitor = Self::visitor();

        for child in children {
            visitor.visit_node(child)?;
        }

        visitor.finish()
    }
}
