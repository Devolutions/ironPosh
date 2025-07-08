use xml_builder::{Attribute, Builder, Declaration, Element, Namespace, RootElement};

fn main() {
    // Create an XML declaration
    let declaration = Declaration::new("1.0", "UTF-8").with_standalone(true);

    // Create an XML element
    let element = Element::new("root")
        .add_attribute(Attribute::new("attr1", "value1"))
        .add_attribute(Attribute::new("attr2", "value2"))
        .add_child(
            Element::new("child1")
                .set_namespace(Namespace::new("ns", "http://example.com/ns"))
                .add_child(
                    Element::new("grandchild").add_attribute(Attribute::new("attr", "value")),
                ),
        )
        .add_child(Element::new("child2"));

    // Create a builder with the declaration and element
    let builder = Builder::new(Some(declaration), RootElement::new(element));

    // Print the XML document
    println!("{}", builder.to_string());
}
