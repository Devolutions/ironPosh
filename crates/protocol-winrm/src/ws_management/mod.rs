pub mod body;
pub mod header;
pub use header::*;

use crate::{
    cores::{
        Attribute, Tag, Time, WsUuid, namespace::Namespace, tag_name::*,
        tag_value::Text,
    },
    soap::{SoapEnvelope, body::SoapBody, header::SoapHeaders},
    ws_addressing::AddressValue,
};

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct WsMan {
    #[builder(default = 512000)]
    max_envelope_size: u32,

    #[builder(default = 180)]
    operation_timeout: u32,

    #[builder(default = "en-CA".to_string())]
    data_locale: String,

    #[builder(default = "en-US".to_string())]
    locale: String,

    #[builder(default = "http://schemas.microsoft.com/powershell/Microsoft.PowerShell".to_string())]
    resource_uri: String,

    #[builder(default = uuid::Uuid::new_v4())]
    session_id: uuid::Uuid,

    to: String,
}

impl WsMan {
    pub fn max_envelope_size(&self) -> u32 {
        self.max_envelope_size
    }
}

#[derive(Debug, Clone)]
pub enum WsAction {
    Create,
    Delete,
    Get,
    Put,
    Command,
    CommandResponse,
    ShellReceive,
    ShellCreate,
}

impl WsAction {
    pub fn as_str(&self) -> &str {
        match self {
            WsAction::Create => "http://schemas.xmlsoap.org/ws/2004/09/transfer/Create",
            WsAction::Delete => "http://schemas.xmlsoap.org/ws/2004/09/transfer/Delete",
            WsAction::Get => "http://schemas.xmlsoap.org/ws/2004/09/transfer/Get",
            WsAction::Put => "http://schemas.xmlsoap.org/ws/2004/09/transfer/Put",
            WsAction::Command => "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Command",
            WsAction::CommandResponse => {
                "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/CommandResponse"
            }
            WsAction::ShellReceive => {
                "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Receive"
            } // See note below
            WsAction::ShellCreate => {
                "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/create"
            }
        }
    }
}

impl WsMan {
    pub fn invoke<'a>(
        &'a self,
        action: WsAction,
        resource_uri: Option<&'a str>,
        resource_body: SoapBody<'a>,
        option_set: Option<header::OptionSetValue>,
        selector_set: Option<header::SelectorSetValue>,
    ) -> Tag<'a, SoapEnvelope<'a>, Envelope> {
        // Generate a unique message ID and operation ID for this request
        let message_id = uuid::Uuid::new_v4();
        let operation_id = uuid::Uuid::new_v4();

        let resource_uri = resource_uri.unwrap_or(self.resource_uri.as_str());

        // Create reply-to address value
        let reply_to_addr = AddressValue {
            url: Tag::new("http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous"),
        };

        // Create the SOAP header with all required fields
        let header = SoapHeaders::builder()
            .action(
                Tag::new(action.as_str().to_owned())
                    .with_name(Action)
                    .with_attribute(Attribute::MustUnderstand(true)),
            )
            .data_locale(
                Tag::new(())
                    .with_attribute(Attribute::MustUnderstand(false))
                    .with_attribute(Attribute::XmlLang(self.data_locale.clone().into())),
            )
            .locale(
                Tag::new(())
                    .with_attribute(Attribute::XmlLang(self.locale.clone().into()))
                    .with_attribute(Attribute::MustUnderstand(false)),
            )
            .max_envelope_size(
                Tag::new(self.max_envelope_size).with_attribute(Attribute::MustUnderstand(true)),
            )
            .resource_uri(Tag::new(resource_uri).with_attribute(Attribute::MustUnderstand(true)))
            .operation_timeout(Time::from(self.operation_timeout))
            .message_id(message_id)
            .to(self.to.as_ref())
            .reply_to(Tag::new(reply_to_addr).with_attribute(Attribute::MustUnderstand(true)))
            .session_id(
                Tag::new(WsUuid(self.session_id)).with_attribute(Attribute::MustUnderstand(false)),
            )
            .operation_id(
                Tag::new(WsUuid(operation_id)).with_attribute(Attribute::MustUnderstand(false)),
            )
            .sequence_id(Tag::new(Text::from("1")).with_attribute(Attribute::MustUnderstand(false)))
            .option_set_opt(option_set.map(Tag::from).map(|t| {
                t.with_declaration(Namespace::XmlSchemaInstance)
                    .with_attribute(Attribute::MustUnderstand(true))
            }))
            .selector_set_opt(selector_set.map(Tag::from))
            .build();

        // TODO: I don't like this design; it's a bit problematic, but I guess I will live with it right now.
        let add_rsp_declaration = resource_body.command_line.is_some();

        // Create the complete SOAP envelope
        let envelope = SoapEnvelope::builder()
            .header(header)
            .body(resource_body)
            .build();

        // Convert to XML using Tag wrapper with proper namespaces

        let mut soap = Tag::<SoapEnvelope, Envelope>::new(envelope)
            .with_declaration(Namespace::SoapEnvelope2003)
            .with_declaration(Namespace::WsAddressing2004)
            .with_declaration(Namespace::DmtfWsmanSchema)
            .with_declaration(Namespace::MsWsmanSchema);

        if add_rsp_declaration {
            soap = soap.with_declaration(Namespace::WsmanShell)
        }

        soap
    }
}
