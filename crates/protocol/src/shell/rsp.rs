use crate::{
    define_custom_tagname, define_tagname, push_element,
    traits::{DeclareNamespaces, PowerShellNamespaceAlias, Tag, tag_value::Text},
};

pub const PWSH_NS: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";
pub const PWSH_NS_ALIAS: &str = "rsp";

// Define tag names for PowerShell remoting shell elements
define_tagname!(ShellId, Some(PWSH_NS));
define_tagname!(Name, Some(PWSH_NS));
define_tagname!(ResourceUri, Some(PWSH_NS));
define_tagname!(Owner, Some(PWSH_NS));
define_tagname!(ClientIP, Some(PWSH_NS));
define_tagname!(ProcessId, Some(PWSH_NS));
define_tagname!(IdleTimeOut, Some(PWSH_NS));
define_tagname!(InputStreams, Some(PWSH_NS));
define_tagname!(OutputStreams, Some(PWSH_NS));
define_tagname!(MaxIdleTimeOut, Some(PWSH_NS));
define_tagname!(Locale, Some(PWSH_NS));
define_tagname!(DataLocale, Some(PWSH_NS));
define_tagname!(CompressionMode, Some(PWSH_NS));
define_tagname!(ProfileLoaded, Some(PWSH_NS));
define_tagname!(Encoding, Some(PWSH_NS));
define_tagname!(BufferMode, Some(PWSH_NS));
define_tagname!(State, Some(PWSH_NS));
define_tagname!(ShellRunTime, Some(PWSH_NS));
define_tagname!(ShellInactivity, Some(PWSH_NS));
define_custom_tagname!(CreationXml, "creationXml", None);

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct Shell<'a> {
    #[builder(setter(into))]
    pub shell_id: Tag<'a, Text<'a>, ShellId>,
    #[builder(default, setter(strip_option, into))]
    pub name: Option<Tag<'a, Text<'a>, Name>>,
    #[builder(default, setter(strip_option, into))]
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceUri>>,
    #[builder(default, setter(strip_option, into))]
    pub owner: Option<Tag<'a, Text<'a>, Owner>>,
    #[builder(default, setter(strip_option, into))]
    pub client_ip: Option<Tag<'a, Text<'a>, ClientIP>>,
    #[builder(default, setter(strip_option, into))]
    pub process_id: Option<Tag<'a, Text<'a>, ProcessId>>,
    #[builder(default, setter(strip_option, into))]
    pub idle_time_out: Option<Tag<'a, Text<'a>, IdleTimeOut>>,
    #[builder(default, setter(strip_option, into))]
    pub input_streams: Option<Tag<'a, Text<'a>, InputStreams>>,
    #[builder(default, setter(strip_option, into))]
    pub output_streams: Option<Tag<'a, Text<'a>, OutputStreams>>,
    #[builder(default, setter(strip_option, into))]
    pub max_idle_time_out: Option<Tag<'a, Text<'a>, MaxIdleTimeOut>>,
    #[builder(default, setter(strip_option, into))]
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    #[builder(default, setter(strip_option, into))]
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    #[builder(default, setter(strip_option, into))]
    pub compression_mode: Option<Tag<'a, Text<'a>, CompressionMode>>,
    #[builder(default, setter(strip_option, into))]
    pub profile_loaded: Option<Tag<'a, Text<'a>, ProfileLoaded>>,
    #[builder(default, setter(strip_option, into))]
    pub encoding: Option<Tag<'a, Text<'a>, Encoding>>,
    #[builder(default, setter(strip_option, into))]
    pub buffer_mode: Option<Tag<'a, Text<'a>, BufferMode>>,
    #[builder(default, setter(strip_option, into))]
    pub state: Option<Tag<'a, Text<'a>, State>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_run_time: Option<Tag<'a, Text<'a>, ShellRunTime>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_inactivity: Option<Tag<'a, Text<'a>, ShellInactivity>>,
    #[builder(default, setter(strip_option, into))]
    pub creation_xml:
        Option<DeclareNamespaces<'a, PowerShellNamespaceAlias, Tag<'a, Text<'a>, CreationXml>>>,
}

impl<'a> IntoIterator for Shell<'a> {
    type Item = xml::builder::Element<'a>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let Self {
            shell_id,
            name,
            resource_uri,
            owner,
            client_ip,
            process_id,
            idle_time_out,
            input_streams,
            output_streams,
            max_idle_time_out,
            locale,
            data_locale,
            compression_mode,
            profile_loaded,
            encoding,
            buffer_mode,
            state,
            shell_run_time,
            shell_inactivity,
            creation_xml,
        } = self;

        let mut tags: Vec<Self::Item> = vec![];

        push_element!(
            tags,
            [
                Some(shell_id),
                name,
                resource_uri,
                owner,
                client_ip,
                process_id,
                idle_time_out,
                input_streams,
                output_streams,
                max_idle_time_out,
                locale,
                data_locale,
                compression_mode,
                profile_loaded,
                encoding,
                buffer_mode,
                state,
                shell_run_time,
                shell_inactivity,
                creation_xml
            ]
        );

        tags.into_iter()
    }
}
