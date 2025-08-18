use protocol_macros::{SimpleTagValue, SimpleXmlDeserialize};
use tracing::warn;

use crate::cores::{
    CommandState, DesiredStream, ExitCode, Stream, Tag, TagName, TagValue, Text, tag_value,
};
use xml::{
    XmlError,
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct ReceiveValue<'a> {
    pub desired_stream: Tag<'a, Text<'a>, DesiredStream>,
}

#[derive(Debug, Clone)]
pub enum CommandStateValueState {
    Done,
    Pending,
    Running,
}

impl CommandStateValueState {
    pub fn value(&self) -> &'static str {
        match self {
            CommandStateValueState::Done => {
                "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Done"
            }
            CommandStateValueState::Pending => {
                "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Pending"
            }
            CommandStateValueState::Running => {
                "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Running"
            }
        }
    }
}

impl TryFrom<&str> for CommandStateValueState {
    type Error = XmlError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Done" => {
                Ok(CommandStateValueState::Done)
            }
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Pending" => {
                Ok(CommandStateValueState::Pending)
            }
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Running" => {
                Ok(CommandStateValueState::Running)
            }
            _ => Err(XmlError::GenericError(format!(
                "Unknown CommandStateValueState: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, SimpleTagValue, SimpleXmlDeserialize)]
pub struct CommandStateValue<'a> {
    pub exit_code: Option<Tag<'a, tag_value::I32, ExitCode>>,
}

// ReceiveResponse main structure
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ReceiveResponseValue<'a> {
    pub streams: Vec<Tag<'a, Text<'a>, Stream>>,
    pub command_state: Option<Tag<'a, CommandStateValue<'a>, CommandState>>,
}

impl<'a> TagValue<'a> for ReceiveResponseValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for stream in self.streams {
            element = element.add_child(stream.into_element());
        }

        element
    }
}

pub struct ReceiveResponseVisitor<'a> {
    pub streams: Vec<Tag<'a, Text<'a>, Stream>>,
    pub command_state: Option<Tag<'a, CommandStateValue<'a>, CommandState>>,
}

impl<'a> XmlVisitor<'a> for ReceiveResponseVisitor<'a> {
    type Value = ReceiveResponseValue<'a>;

    fn visit_children(
        &mut self,
        nodes: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for node in nodes {
            match (node.tag_name().name(), node.tag_name().namespace()) {
                (Stream::TAG_NAME, Stream::NAMESPACE) => {
                    let stream = Tag::from_node(node)?;
                    self.streams.push(stream);
                }
                (CommandState::TAG_NAME, CommandState::NAMESPACE) => {
                    let command_state = Tag::from_node(node)?;
                    self.command_state = Some(command_state);
                }
                _ => {
                    warn!(
                        "Unexpected tag in ReceiveResponse: {}",
                        node.tag_name().name()
                    );
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, XmlError> {
        Ok(ReceiveResponseValue {
            streams: self.streams,
            command_state: self.command_state,
        })
    }
}

impl<'a> XmlDeserialize<'a> for ReceiveResponseValue<'a> {
    type Visitor = ReceiveResponseVisitor<'a>;

    fn visitor() -> Self::Visitor {
        ReceiveResponseVisitor {
            streams: Vec::new(),
            command_state: None,
        }
    }
}
