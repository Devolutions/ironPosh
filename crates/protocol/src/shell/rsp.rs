use crate::{
    push_elements,
    traits::{Tag, TagList, tag_name::*, tag_value::Text},
};

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
    pub creation_xml: Option<Tag<'a, TagList<'a>, CreationXml>>,
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

        push_elements!(
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
