use tracing::warn;

use crate::cores::{
    Tag, TagName, TagValue, Text,
    tag_name::{Arguments, Command},
};

#[derive(Debug, Clone)]
pub struct CommandLineValue {
    pub command: Option<String>,
    pub arguments: Vec<String>,
}

impl<'a> TagValue<'a> for CommandLineValue {
    fn append_to_element(self, mut element: xml::builder::Element) -> xml::builder::Element {
        let command_element = self
            .command
            .map(|cmd| {
                Tag::from_name(Command)
                    .with_value(Text::from(cmd))
                    .into_element()
            })
            .unwrap_or(Tag::from_name(Command).with_value(()).into_element());

        element = element.add_child(command_element);

        for arg in self.arguments {
            let arg_element = Tag::from_name(Arguments)
                .with_value(Text::from(arg))
                .into_element();
            element = element.add_child(arg_element);
        }

        element
    }
}

pub struct CommandLineValueVisitor {
    command_line: Option<String>,
    arguments: Vec<String>,
}

impl<'a> xml::parser::XmlVisitor<'a> for CommandLineValueVisitor {
    type Value = CommandLineValue;

    fn visit_children(
        &mut self,
        nodes: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for node in nodes {
            match (node.tag_name().name(), node.tag_name().namespace()) {
                (Command::TAG_NAME, Command::NAMESPACE) => {
                    let cmd_text = node.text().map(|t| t.to_string());
                    self.command_line = cmd_text;
                }
                (Arguments::TAG_NAME, Arguments::NAMESPACE) => {
                    if let Some(text) = node.text() {
                        self.arguments.push(text.to_string());
                    }
                }
                _ => {
                    warn!(
                        "Unexpected tag in CommandLineValue: {}",
                        node.tag_name().name()
                    );
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(CommandLineValue {
            command: self.command_line,
            arguments: self.arguments,
        })
    }
}

impl<'a> xml::parser::XmlDeserialize<'a> for CommandLineValue {
    type Visitor = CommandLineValueVisitor;

    fn visitor() -> Self::Visitor {
        CommandLineValueVisitor {
            command_line: None,
            arguments: Vec::new(),
        }
    }
}

