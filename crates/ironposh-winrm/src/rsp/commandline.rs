use ironposh_xml::mapping::{FromXml, NodeExt};

use crate::cores::{
    Tag, TagName, TagValue, Text,
    tag_name::{Arguments, Command},
};

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
        let command_element = self.command.map_or_else(
            || Tag::from_name(Command).with_value(()).into_element(),
            |cmd| {
                Tag::from_name(Command)
                    .with_value(Text::from(cmd))
                    .into_element()
            },
        );

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

impl<'a> FromXml<'a> for CommandLineValue {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        let mut command = None;
        let mut arguments = Vec::new();
        for child in node.children() {
            if child.is_element_named(Command::NAMESPACE, Command::TAG_NAME) {
                command = child.text().map(ToString::to_string);
            } else if child.is_element_named(Arguments::NAMESPACE, Arguments::TAG_NAME)
                && let Some(text) = child.text()
            {
                arguments.push(text.to_string());
            }
        }
        Ok(CommandLineValue { command, arguments })
    }
}
