use crate::cores::Attribute;
use crate::cores::Tag;
use crate::cores::TagList;
use crate::cores::tag;
use crate::cores::tag_list;
use crate::cores::tag_name;
use crate::cores::tag_name::*;

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct SessionCapability<'a> {
    ref_id: usize,
    protocol_version: &'a str,
    powershell_remoting_version: &'a str,
    serialization_version: &'a str,
    timezone: &'a str,
}

impl<'a> SessionCapability<'a> {
    pub fn to_tag(
        &self,
    ) -> tag::Tag<'_, tag_name::MSWrapper<'_, tag_list::TagList<'_>>, tag_name::Obj> {
        let tag = Tag::from_name(Obj)
            .with_attribute(crate::cores::Attribute::RefId(
                self.ref_id.to_string().into(),
            ))
            .with_value(MSWrapper::new(
                TagList::new()
                    .with_tag(
                        Tag::new(self.protocol_version)
                            .with_attribute(Attribute::N("protocolversion".into()))
                            .with_name(Version)
                            .into(),
                    )
                    .with_tag(
                        Tag::new(self.powershell_remoting_version)
                            .with_attribute(Attribute::N("PSVersion".into()))
                            .with_name(Version)
                            .into(),
                    )
                    .with_tag(
                        Tag::new(self.serialization_version)
                            .with_attribute(Attribute::N("SerializationVersion".into()))
                            .with_name(Version)
                            .into(),
                    )
                    .with_tag(
                        Tag::new(self.timezone)
                            .with_name(BA)
                            .with_attribute(Attribute::N("TimeZone".into()))
                            .into(),
                    )
                    .into(),
            ));

        tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_capability_to_tag() {
        let capability = SessionCapability::builder()
            .ref_id(1)
            .protocol_version("2.0")
            .powershell_remoting_version("1.0")
            .serialization_version("1.1")
            .timezone("UTC")
            .build();

        let tag = capability.to_tag();

        // Test that the tag creation works without formatting
        println!("Tag created successfully!");

        // Try to get the element
        let element = tag.into_element();
        println!("Element created successfully!");

        // Now try to format it - this is where the error might occur
        let string = format!("{}", element);
        println!("{}", string);
    }
}
