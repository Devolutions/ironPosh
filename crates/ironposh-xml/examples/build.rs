use ironposh_xml::builder::{Attribute, Builder, Declaration, Element};

const NS1: &str = "http://example.com/ns1";
const NS2: &str = "http://example.com/ns2";
const NS1_ALIAS: &str = "ns1";
const NS2_ALIAS: &str = "ns2";

fn main() {
    // Create an XML declaration
    let declaration = Declaration::new("1.0", "UTF-8").with_standalone(true);

    // Create an XML element
    let element = Element::new("root")
        .add_attribute(Attribute::new("attr1", "value1"))
        .add_attribute(Attribute::new("attr2", "value2"))
        .add_child(
            Element::new("child1").set_namespace(NS1).add_child(
                Element::new("grandchild")
                    .add_attribute(Attribute::new("attr", "value"))
                    .set_namespace(NS2),
            ),
        )
        .add_child(
            Element::new("child2")
                .set_namespace(NS2)
                .add_namespace_declaration(NS2, Some(NS2_ALIAS))
                .set_text("Text content for child2")
                .add_attribute(Attribute::new("attr2", "value2").set_namespace(NS1)),
        )
        .add_child(Element::new("child3"));

    // Create a builder with the declaration and element
    let builder = Builder::new(
        Some(declaration),
        element.add_namespace_declaration(NS1, Some(NS1_ALIAS)),
    );

    // Print the XML document
    println!("{}", builder.to_xml_string().unwrap());
}
