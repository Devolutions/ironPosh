use xml::parser::{XmlDeserialize, XmlVisitor};

use crate::{
    cores::{Tag, TagList, TagName, tag_name::*, tag_value::Text},
    rsp::rsp::ShellValue,
};

#[macro_export]
macro_rules! define_any_tag {
    ($enum_name:ident, $visitor_name:ident, $(($variant:ident, $tag_name:ty, $tag_type:ty)),* $(,)?) => {
        #[derive(Debug, Clone)]
        pub enum $enum_name<'a> {
            $($variant($tag_type),)*
        }

        $(
            impl<'a> std::convert::TryInto<$tag_type> for AnyTag<'a> {
                type Error = xml::XmlError;

                fn try_into(self) -> Result<$tag_type, Self::Error> {
                    match self {
                        $enum_name::$variant(tag) => Ok(tag),
                        _ => Err(xml::XmlError::InvalidXml(format!(
                            "Cannot convert {:?} to any tag type",
                            self
                        ))),
                    }
                }
            }

            impl<'a> std::convert::From<$tag_type> for $enum_name<'a> {
                fn from(tag: $tag_type) -> Self {
                    $enum_name::$variant(tag)
                }
            }
        )*


        impl<'a> $enum_name<'a> {
            pub fn into_element(self) -> xml::builder::Element<'a> {
                match self {
                    $($enum_name::$variant(tag) => tag.into_element(),)*
                }
            }
        }

        pub struct $visitor_name<'a> {
            tag: Option<$enum_name<'a>>,
        }

        impl<'a> XmlVisitor<'a> for $visitor_name<'a> {
            type Value = $enum_name<'a>;

            fn visit_children(
                &mut self,
                node: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
            ) -> Result<(), xml::XmlError> {
                Err(xml::XmlError::InvalidXml(format!(
                    "Expected a single tag, found {} children",
                    node.count()
                )))
            }

            fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
                match node.tag_name().name() {
                    $(
                        <$tag_name>::TAG_NAME => {
                            let tag = <$tag_type>::from_node(node)?;
                            self.tag = Some($enum_name::$variant(tag));
                        }
                    )*
                    _ => {
                        return Err(xml::XmlError::InvalidXml(format!(
                            "Unknown tag: {}",
                            node.tag_name().name()
                        )));
                    }
                };

                Ok(())
            }

            fn finish(self) -> Result<Self::Value, xml::XmlError> {
                self.tag
                    .ok_or(xml::XmlError::InvalidXml("No valid tag found".to_string()))
            }
        }

        impl<'a> XmlDeserialize<'a> for $enum_name<'a> {
            type Visitor = $visitor_name<'a>;

            fn visitor() -> Self::Visitor {
                $visitor_name { tag: None }
            }

            fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
                xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
            }
        }
    };
}

// Define the `AnyTag` enum for the following purposes:
// Right now, since we are having all tag and tag name definations defined in compile time (i.e Sized, no dynamic trait objects),
// Hence, for the purpose of having a dynamic representation of any tag, we define `AnyTag`.
// This will allow us to have a single enum that can represent any tag type, which can be useful for generic processing of XML documents.
// while perserving the compile-time safety of tag definitions.
define_any_tag!(
    AnyTag,
    AnyTagVisitor,
    // SOAP elements
    (Envelope, Envelope, Tag<'a, TagList<'a>, Envelope>),
    (Header, Header, Tag<'a, TagList<'a>, Header>),
    (Body, Body, Tag<'a, TagList<'a>, Body>),
    // WS-Addressing headers
    (Action, Action, Tag<'a, Text<'a>, Action>),
    (To, To, Tag<'a, Text<'a>, To>),
    (MessageID, MessageID, Tag<'a, Text<'a>, MessageID>),
    (RelatesTo, RelatesTo, Tag<'a, Text<'a>, RelatesTo>),
    (ReplyTo, ReplyTo, Tag<'a, TagList<'a>, ReplyTo>),
    (FaultTo, FaultTo, Tag<'a, Text<'a>, FaultTo>),
    (From, From, Tag<'a, Text<'a>, From>),
    (Address, Address, Tag<'a, Text<'a>, Address>),
    // PowerShell remoting shell elements
    (ShellId, ShellId, Tag<'a, Text<'a>, ShellId>),
    (Shell, Shell, Tag<'a, ShellValue<'a>, Shell>),
    (Name, Name, Tag<'a, Text<'a>, Name>),
    (ResourceUri, ResourceUri, Tag<'a, Text<'a>, ResourceUri>),
    (Owner, Owner, Tag<'a, Text<'a>, Owner>),
    (ClientIP, ClientIP, Tag<'a, Text<'a>, ClientIP>),
    (ProcessId, ProcessId, Tag<'a, Text<'a>, ProcessId>),
    (IdleTimeOut, IdleTimeOut, Tag<'a, Text<'a>, IdleTimeOut>),
    (InputStreams, InputStreams, Tag<'a, Text<'a>, InputStreams>),
    (
        OutputStreams,
        OutputStreams,
        Tag<'a, Text<'a>, OutputStreams>
    ),
    (
        MaxIdleTimeOut,
        MaxIdleTimeOut,
        Tag<'a, Text<'a>, MaxIdleTimeOut>
    ),
    (Locale, Locale, Tag<'a, Text<'a>, Locale>),
    (DataLocale, DataLocale, Tag<'a, Text<'a>, DataLocale>),
    (
        CompressionMode,
        CompressionMode,
        Tag<'a, Text<'a>, CompressionMode>
    ),
    (
        ProfileLoaded,
        ProfileLoaded,
        Tag<'a, Text<'a>, ProfileLoaded>
    ),
    (Encoding, Encoding, Tag<'a, Text<'a>, Encoding>),
    (BufferMode, BufferMode, Tag<'a, Text<'a>, BufferMode>),
    (State, State, Tag<'a, Text<'a>, State>),
    (ShellRunTime, ShellRunTime, Tag<'a, Text<'a>, ShellRunTime>),
    (
        ShellInactivity,
        ShellInactivity,
        Tag<'a, Text<'a>, ShellInactivity>
    ),
    (CreationXml, CreationXml, Tag<'a, TagList<'a>, CreationXml>),
    (Version, Version, Tag<'a, Text<'a>, Version>),
    (BA, BA, Tag<'a, Text<'a>, BA>),
    // PowerShell Serialization Format
    (I32Tag, I32, Tag<'a, Text<'a>, I32>),
    (TN, TN, Tag<'a, TagList<'a>, TN>),
    (T, T, Tag<'a, Text<'a>, T>),
    (ToString, ToString, Tag<'a, Text<'a>, ToString>),
    (DCT, DCT, Tag<'a, TagList<'a>, DCT>),
    (En, En, Tag<'a, TagList<'a>, En>),
    (Key, Key, Tag<'a, Text<'a>, Key>),
    (Value, Value, Tag<'a, Text<'a>, Value>),
    (Nil, Nil, Tag<'a, Text<'a>, Nil>),
    (B, B, Tag<'a, Text<'a>, B>),
    (S, S, Tag<'a, Text<'a>, S>),
    // Complex objects
    (Obj, Obj, Tag<'a, TagList<'a>, Obj>),
    (MS, MS, Tag<'a, TagList<'a>, MS>),
);
