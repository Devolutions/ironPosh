pub use roxmltree::*;

impl<'a> From<roxmltree::Node<'a, 'a>> for crate::builder::Element<'a> {
    fn from(node: roxmltree::Node<'a, 'a>) -> Self {
        let mut element = crate::builder::Element::new(node.tag_name().name());

        for child in node.children() {
            if child.is_element() {
                let child_element = crate::builder::Element::from(child);
                element = element.add_child(child_element);
            } else if child.is_text() {
                let text_content = child.text().unwrap_or_default();
                element = element.set_text(text_content);
            } else {
                // Handle other node types if necessary (e.g., comments, processing instructions)
                // For now, we will ignore them.
            }
        }

        element
    }
}
