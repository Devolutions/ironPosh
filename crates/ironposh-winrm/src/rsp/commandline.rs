use ironposh_xml::mapping::{FromXml, NodeExt};

use crate::cores::tag_value::leaf_text;
use crate::cores::{ArgumentsTag, CommandTag, Tag, TagName, TagValue, Text};
use crate::tag;

tag!(CommandLine = CommandLineValue => WsmanShell);

#[derive(Debug, Clone)]
pub struct CommandLineValue {
    pub command: Option<String>,
    pub arguments: Vec<String>,
}

impl TagValue<'_> for CommandLineValue {
    fn append_to_element(
        self,
        mut element: ironposh_xml::builder::Element,
    ) -> ironposh_xml::builder::Element {
        // `Command` carries either nothing (`<Command/>`) or text, so it's built
        // explicitly from the marker rather than a single-value alias.
        let command_element = self.command.map_or_else(
            || Tag::from_name(CommandTag).with_value(()).into_element(),
            |cmd| {
                Tag::from_name(CommandTag)
                    .with_value(Text::from(cmd))
                    .into_element()
            },
        );

        element = element.add_child(command_element);

        for arg in self.arguments {
            let arg_element = Tag::from_name(ArgumentsTag)
                .with_value(Text::from(arg))
                .into_element();
            element = element.add_child(arg_element);
        }

        element
    }
}

impl<'a> FromXml<'a> for CommandLineValue {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        let mut command = None;
        let mut seen_command = false;
        let mut arguments = Vec::new();
        for child in node.children() {
            if child.is_element_named(CommandTag::NAMESPACE, CommandTag::TAG_NAME) {
                if seen_command {
                    return Err(ironposh_xml::XmlError::InvalidXml(
                        "duplicate <Command> in CommandLine".into(),
                    ));
                }
                seen_command = true;
                // An empty `<Command/>` is "no command", matching how the
                // serializer's `None` path emits it.
                let text = leaf_text(child)?;
                command = (!text.is_empty()).then(|| text.to_string());
            } else if child.is_element_named(ArgumentsTag::NAMESPACE, ArgumentsTag::TAG_NAME) {
                arguments.push(leaf_text(child)?.to_string());
            }
        }
        Ok(Self { command, arguments })
    }
}
