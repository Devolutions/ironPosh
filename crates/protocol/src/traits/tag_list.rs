use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

use super::tag_value::TagValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagList<'a, T>
where
    T: TagValue<'a>,
{
    items: Vec<T>,
    _phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T> TagList<'a, T>
where
    T: TagValue<'a>,
{
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn into_items(self) -> Vec<T> {
        self.items
    }
}

impl<'a, T> From<Vec<T>> for TagList<'a, T>
where
    T: TagValue<'a>,
{
    fn from(items: Vec<T>) -> Self {
        Self::new(items)
    }
}

impl<'a, T> std::ops::Deref for TagList<'a, T>
where
    T: TagValue<'a>,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<'a, T> IntoIterator for TagList<'a, T>
where
    T: TagValue<'a>,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T> TagValue<'a> for TagList<'a, T>
where
    T: TagValue<'a>,
{
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name);

        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }

        for item in self.items {
            let child = item.into_element(name, namespace);
            element = element.add_child(child);
        }

        element
    }
}

pub struct TagListVisitor<'a, T>
where
    T: XmlDeserialize<'a> + TagValue<'a>,
{
    items: Vec<T>,
    _phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T> XmlVisitor<'a> for TagListVisitor<'a, T>
where
    T: XmlDeserialize<'a> + TagValue<'a>,
{
    type Value = TagList<'a, T>;

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        Ok(())
    }

    fn visit_children(
        &mut self,
        children: xml::parser::Children<'a, 'a>,
    ) -> Result<(), xml::XmlError<'a>> {
        // Try to deserialize each child node as T
        for child in children {
            if child.is_element() {
                match T::from_node(child) {
                    Ok(item) => self.items.push(item),
                    Err(_) => continue, // Skip nodes that can't be deserialized as T
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        Ok(TagList::new(self.items))
    }
}

impl<'a, T> XmlDeserialize<'a> for TagList<'a, T>
where
    T: XmlDeserialize<'a> + TagValue<'a>,
{
    type Visitor = TagListVisitor<'a, T>;

    fn visitor() -> Self::Visitor {
        TagListVisitor {
            items: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
