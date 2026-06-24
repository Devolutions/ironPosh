use ironposh_macros::{FromXml, SimpleTagValue};

use crate::cores::{
    DesiredStream, DesiredStreamTag, ExitCode, Stream, StreamTag, TagName, TagValue,
};
use crate::tag;
use ironposh_xml::{
    XmlError,
    builder::Element,
    mapping::{FromXml, NodeExt},
};

tag!(Receive = ReceiveValue<'a> => WsmanShell);
tag!(ReceiveResponse = ReceiveResponseValue<'a> => WsmanShell);
tag!(CommandState = CommandStateValue<'a> => WsmanShell);

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ReceiveValue<'a> {
    pub desired_streams: Vec<DesiredStream<'a>>,
}

impl<'a> TagValue<'a> for ReceiveValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for stream in self.desired_streams {
            element = element.add_child(stream.into_element());
        }
        element
    }
}

impl<'a> FromXml<'a> for ReceiveValue<'a> {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, XmlError> {
        let mut desired_streams = Vec::new();
        for child in node.children() {
            if child.is_element_named(DesiredStreamTag::NAMESPACE, DesiredStreamTag::TAG_NAME) {
                desired_streams.push(DesiredStream::from_xml(child)?);
            }
        }
        Ok(ReceiveValue { desired_streams })
    }
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
            Self::Done => {
                "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Done"
            }
            Self::Pending => {
                "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Pending"
            }
            Self::Running => {
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
                Ok(Self::Done)
            }
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Pending" => {
                Ok(Self::Pending)
            }
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell/CommandState/Running" => {
                Ok(Self::Running)
            }
            _ => Err(XmlError::GenericError(format!(
                "Unknown CommandStateValueState: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, SimpleTagValue, FromXml)]
pub struct CommandStateValue<'a> {
    pub exit_code: Option<ExitCode<'a>>,
}

// ReceiveResponse main structure
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ReceiveResponseValue<'a> {
    pub streams: Vec<Stream<'a>>,
    pub command_state: Option<CommandState<'a>>,
}

impl<'a> TagValue<'a> for ReceiveResponseValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for stream in self.streams {
            element = element.add_child(stream.into_element());
        }

        element
    }
}

impl<'a> FromXml<'a> for ReceiveResponseValue<'a> {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, XmlError> {
        let mut streams = Vec::new();
        let mut command_state = None;
        for child in node.children() {
            if child.is_element_named(StreamTag::NAMESPACE, StreamTag::TAG_NAME) {
                streams.push(Stream::from_xml(child)?);
            } else if child.is_element_named(CommandStateTag::NAMESPACE, CommandStateTag::TAG_NAME)
            {
                command_state = Some(CommandState::from_xml(child)?);
            }
        }
        Ok(ReceiveResponseValue {
            streams,
            command_state,
        })
    }
}
