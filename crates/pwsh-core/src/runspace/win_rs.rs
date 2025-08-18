use base64::Engine;
use protocol_winrm::{
    cores::{
        Attribute, DesiredStream, Receive, Shell, Tag, Time,
    },
    rsp::{
        commandline::CommandLineValue,
        receive::ReceiveValue,
        rsp::ShellValue,
    },
    soap::{SoapEnvelope, body::SoapBody},
    ws_management::{self, OptionSetValue, SelectorSetValue, WsMan},
};
use uuid::Uuid;
use xml::builder::Element;

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct WinRunspace {
    #[builder(default = "stdin pr".to_string())]
    input_streams: String,
    #[builder(default = "stdout".to_string())]
    output_streams: String,
    #[builder(default, setter(strip_option))]
    environment: Option<std::collections::HashMap<String, String>>,
    #[builder(default, setter(strip_option))]
    idle_time_out: Option<f64>,
    #[builder(default, setter(strip_option))]
    name: Option<String>,

    #[builder(default = "http://schemas.microsoft.com/powershell/Microsoft.PowerShell".to_string())]
    resource_uri: String,

    #[builder(default = uuid::Uuid::new_v4())]
    id: uuid::Uuid,

    #[builder(default)]
    no_profile: Option<bool>,

    #[builder(default)]
    codepage: Option<u32>,

    #[builder(default)]
    shell_id: Option<String>,
    #[builder(default)]
    owner: Option<String>,
    #[builder(default)]
    client_ip: Option<String>,
    #[builder(default)]
    shell_run_time: Option<String>,
    #[builder(default)]
    shell_inactivity: Option<String>,

    #[builder(default)]
    selector_set: SelectorSetValue,

    #[builder(default)]
    opened: bool,
}

impl WinRunspace {
    pub fn open<'a>(
        &'a self,
        ws_man: &'a WsMan,
        option_set: Option<OptionSetValue>,
        open_content: &'a str,
    ) -> impl Into<Element<'a>> {
        let shell = Tag::from_name(Shell)
            .with_attribute(protocol_winrm::cores::Attribute::ShellId(
                self.id.to_string().into(),
            ))
            .with_attribute(protocol_winrm::cores::Attribute::Name(
                self.name.as_deref().unwrap_or("Runspace1").into(),
            ))
            .with_declaration(protocol_winrm::cores::Namespace::WsmanShell);

        let shell_value = ShellValue::builder()
            .input_streams(self.input_streams.as_ref())
            .output_streams(self.output_streams.as_ref())
            .idle_time_out_opt(self.idle_time_out.map(Time).map(Tag::new))
            .creation_xml(
                Tag::new(open_content)
                    .with_declaration(protocol_winrm::cores::Namespace::PowerShellRemoting),
            )
            .build();

        let shell = shell.with_value(shell_value);

        let mut option_set = option_set.unwrap_or_default();

        if let Some(profile) = self.no_profile {
            option_set = option_set.add_option("WINRS_NOPROFILE", profile.to_string());
        }

        if let Some(codepage) = self.codepage {
            option_set = option_set.add_option("WINRS_CODEPAGE", codepage.to_string());
        }

        ws_man.invoke(
            ws_management::WsAction::Create,
            None,
            SoapBody::builder().shell(shell).build(),
            Some(option_set),
            None,
        )
    }

    pub fn fire_receive<'a>(
        &'a self,
        ws_man: &'a WsMan,
        stream: Option<&'a str>,
        command_id: Option<&'a str>,
    ) -> impl Into<Element<'a>> {
        let stream = stream.unwrap_or("stdout");

        let desired_stream = Tag::new(stream).with_name(DesiredStream);

        let desired_stream = if let Some(command_id) = command_id {
            desired_stream.with_attribute(protocol_winrm::cores::Attribute::CommandId(
                command_id.into(),
            ))
        } else {
            desired_stream
        };

        let receive = ReceiveValue::builder()
            .desired_stream(desired_stream)
            .build();

        let receive_tag = Tag::from_name(Receive)
            .with_value(receive)
            .with_declaration(protocol_winrm::cores::Namespace::WsmanShell);

        let option_set = OptionSetValue::default()
            .add_option("WSMAN_CMDSHELL_OPTION_KEEPALIVE", true.to_string());

        let selector_set = self
            .shell_id
            .as_ref()
            .map(|shell_id| SelectorSetValue::new().add_selector("ShellId", shell_id));

        ws_man.invoke(
            ws_management::WsAction::ShellReceive,
            Some(&self.resource_uri),
            SoapBody::builder().receive(receive_tag).build(),
            Some(option_set),
            selector_set,
        )
    }

    pub fn accept_receive_response<'a>(
        &mut self,
        soap_envelope: &SoapEnvelope<'a>,
    ) -> Result<Vec<Vec<u8>>, crate::PwshCoreError> {
        let receive_response = &soap_envelope
            .body
            .as_ref()
            .receive_response
            .as_ref()
            .ok_or(crate::PwshCoreError::InvalidResponse(
                "No ReceiveResponse found in response".into(),
            ))?;

        let streams = receive_response
            .value
            .streams
            .iter()
            .map(|stream| stream.value.as_ref())
            .map(|stream| base64::engine::general_purpose::STANDARD.decode(stream))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                crate::PwshCoreError::InvalidResponse("Failed to decode streams".into())
            })?;

        Ok(streams)
    }

    pub fn accept_create_response<'a>(
        &mut self,
        soap_envelop: &SoapEnvelope<'a>,
    ) -> Result<(), crate::PwshCoreError> {
        let shell = &soap_envelop.body.as_ref().shell.as_ref().ok_or(
            crate::PwshCoreError::InvalidResponse("No shell found in response".into()),
        )?;
        let shell_id = shell.as_ref().shell_id.as_ref().map(|id| id.clone_value());
        let resource_uri = &shell.as_ref().resource_uri;
        let owner = &shell.as_ref().owner;
        let client_ip = &shell.as_ref().client_ip;
        let idle_time_out = &shell.as_ref().idle_time_out;
        let output_stream = &shell.as_ref().output_streams;
        let shell_run_time = &shell.as_ref().shell_run_time;
        let shell_inactivity = &shell.as_ref().shell_inactivity;

        self.shell_id = shell_id.map(|s| s.as_ref().to_string());
        self.owner = owner.as_ref().map(|o| o.value.as_ref().to_string());
        self.client_ip = client_ip.as_ref().map(|c| c.value.as_ref().to_string());
        self.idle_time_out = idle_time_out.as_ref().map(|t| t.value.0);
        self.output_streams = output_stream
            .as_ref()
            .map(|o| o.value.as_ref().to_string())
            .unwrap_or_else(|| "stdout".to_string());

        self.resource_uri = resource_uri
            .as_ref()
            .map(|r| r.value.as_ref().to_string())
            .unwrap_or_else(|| self.resource_uri.clone());

        self.shell_run_time = shell_run_time
            .as_ref()
            .map(|t| t.value.as_ref().to_string());

        self.shell_inactivity = shell_inactivity
            .as_ref()
            .map(|t| t.value.as_ref().to_string());

        let resource_created = soap_envelop.body.as_ref().resource_created.as_ref().ok_or(
            crate::PwshCoreError::InvalidResponse("No ResourceCreated found in response".into()),
        )?;

        let reference_parameters = resource_created.as_ref().reference_parameters.as_ref();

        let selector_set = &reference_parameters.selector_set;

        self.selector_set = selector_set.value.clone();

        self.opened = true;

        Ok(())
    }

    pub(crate) fn create_pipeline_request<'a>(
        &'a self,
        connection: &'a WsMan,
        command_id: uuid::Uuid,
        arguments: Vec<String>,
        executable: Option<String>,
        no_shell: Option<bool>,
    ) -> Result<impl Into<Element<'a>>, crate::PwshCoreError> {
        let command_line = CommandLineValue {
            command: executable,
            arguments,
        };

        let request = connection.invoke(
            ws_management::WsAction::Command,
            Some(self.resource_uri.as_ref()),
            SoapBody::builder()
                .command_line(
                    Tag::new(command_line)
                        .with_attribute(Attribute::CommandId(command_id.to_string().into())),
                )
                .build(),
            Some(OptionSetValue::default().add_option(
                "WINRS_SKIP_CMD_SHELL",
                no_shell.unwrap_or_default().to_string(),
            )),
            self.selector_set.clone().into(),
        );

        Ok(request)
    }

    pub fn accept_commannd_response<'a>(
        &mut self,
        soap_envelope: SoapEnvelope<'a>,
    ) -> Result<Uuid, crate::PwshCoreError> {
        let command_id = soap_envelope
            .body
            .as_ref()
            .command_response
            .as_ref()
            .ok_or(crate::PwshCoreError::InvalidResponse(
                "No CommandResponse found in response".into(),
            ))?
            .as_ref()
            .as_ref();

        Ok(command_id.0)
    }
}
