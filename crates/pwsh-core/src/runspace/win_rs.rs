use protocol_winrm::{
    cores::{Shell, Tag, U32, anytag::AnyTag},
    rsp::rsp::ShellValue,
    ws_management::{self, OptionSetValue},
};
use xml::builder::Element;

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct WinRunspace<'wsman> {
    #[builder(default = "stdin pr".to_string())]
    input_streams: String,
    #[builder(default = "stdout".to_string())]
    output_streams: String,
    #[builder(default, setter(strip_option))]
    environment: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(strip_option))]
    idle_time_out: Option<u32>,
    #[builder(default, setter(strip_option))]
    name: Option<String>,

    #[builder(default = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/cmd".to_string())]
    resource_uri: String,

    #[builder(default = uuid::Uuid::new_v4())]
    id: uuid::Uuid,

    #[builder(default)]
    no_profile: Option<bool>,

    #[builder(default)]
    codepage: Option<u32>,

    ws_man: &'wsman protocol_winrm::ws_management::WsMan,
}

impl<'wsman> WinRunspace<'wsman> {
    pub fn open<'a>(
        &'wsman self,
        option_set: Option<OptionSetValue>,
        open_content: &'a str,
    ) -> impl Into<Element<'a>>
    where
        'wsman: 'a,
    {
        let shell = Tag::from_name(Shell)
            .with_attribute(protocol_winrm::cores::Attribute::ShellId(
                self.id.to_string().into(),
            ))
            .with_declaration(protocol_winrm::cores::Namespace::PowerShellRemoting);

        let shell_value = ShellValue::builder()
            .input_streams(self.input_streams.as_ref())
            .output_streams(self.output_streams.as_ref())
            .idle_time_out_opt(self.idle_time_out.map(Tag::from))
            .creation_xml(open_content)
            .build();

        let shell = shell.with_value(shell_value);

        let mut option_set = option_set.unwrap_or_default();

        if let Some(profile) = self.no_profile {
            option_set = option_set.add_option("WINRS_NOPROFILE", profile.to_string());
        }

        if let Some(codepage) = self.codepage {
            option_set = option_set.add_option("WINRS_CODEPAGE", codepage.to_string());
        }

        self.ws_man.invoke(
            ws_management::WsAction::Create,
            None,
            Some(AnyTag::Shell(shell)),
            Some(option_set),
            None,
        )
    }
}
