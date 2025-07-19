use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

pub trait TagValue<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Text<'a>(&'a str);

impl<'a> From<&'a str> for Text<'a> {
    fn from(value: &'a str) -> Self {
        Text(value)
    }
}

impl<'a> From<Text<'a>> for &'a str {
    fn from(val: Text<'a>) -> Self {
        val.0
    }
}

impl<'a> TagValue<'a> for Text<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element.set_text(self.0.as_ref())
    }
}

impl<'a> TagValue<'a> for () {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element
    }
}

pub struct TextVisitor<'a> {
    value: Option<Text<'a>>,
}

impl<'a> XmlVisitor<'a> for TextVisitor<'a> {
    type Value = Text<'a>;

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        let child_nodes: Vec<_> = children.collect();

        // Validate there's only one child node
        if child_nodes.len() != 1 {
            return Err(xml::XmlError::InvalidXml(format!(
                "Expected exactly one text node, found {} children",
                child_nodes.len()
            )));
        }

        let child = child_nodes.first().ok_or_else(|| {
            xml::XmlError::InvalidXml("Expected at least one child node".to_string())
        })?;

        // Validate that child node is a text node
        if !child.is_text() {
            return Err(xml::XmlError::InvalidXml(
                "Expected text node, found non-text child".to_string(),
            ));
        }

        if let Some(text) = child.text() {
            self.value = Some(Text(text.trim()));
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        self.value.ok_or(xml::XmlError::InvalidXml(
            "No text found in the node".to_string(),
        ))
    }
}

impl<'a> XmlDeserialize<'a> for Text<'a> {
    type Visitor = TextVisitor<'a>;

    fn visitor() -> Self::Visitor {
        TextVisitor { value: None }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

pub struct EmptyVisitor;

impl<'a> XmlVisitor<'a> for EmptyVisitor {
    type Value = Empty;

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        let child_count = children.count();

        if child_count != 0 {
            return Err(xml::XmlError::InvalidXml(format!(
                "Expected empty tag with no children, found {child_count} children"
            )));
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        Ok(Empty)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empty;

impl<'a> XmlDeserialize<'a> for Empty {
    type Visitor = EmptyVisitor;

    fn visitor() -> Self::Visitor {
        EmptyVisitor
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}

impl<'a> TagValue<'a> for Empty {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element
    }
}
