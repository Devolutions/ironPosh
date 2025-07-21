use crate::cores::{Attribute, Tag, TagList};
use crate::cores::tag_name::*;
use crate::cores::tag_value::Text;

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct PSThreadOptions<'a> {
    #[builder(default = 2)]
    ref_id: usize,
    #[builder(default = "Default")]
    to_string_value: &'a str,
    #[builder(default = 0)]
    int_value: i32,
}

impl<'a> Default for PSThreadOptions<'a> {
    fn default() -> Self {
        PSThreadOptions {
            ref_id: 2,
            to_string_value: "Default",
            int_value: 0,
        }
    }
}

impl<'a> PSThreadOptions<'a> {
    pub fn to_tag(&self) -> Tag<'_, TagList<'_>, Obj> {
        let int_value_str = self.int_value.to_string();
        let ref_id_str = self.ref_id.to_string();
        
        let type_names = TagList::new()
            .with_tag(
                Tag::new(Text::from("System.Management.Automation.Runspaces.PSThreadOptions"))
                    .with_name(T)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from("System.Enum"))
                    .with_name(T)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from("System.ValueType"))
                    .with_name(T)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from("System.Object"))
                    .with_name(T)
                    .into(),
            );

        let tn_tag = Tag::new(type_names)
            .with_attribute(Attribute::RefId("0".into()))
            .with_name(TN);

        let ms_content = TagList::new()
            .with_tag(tn_tag.into())
            .with_tag(
                Tag::new(Text::from(self.to_string_value))
                    .with_name(ToString)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from(int_value_str))
                    .with_name(I32)
                    .into(),
            );

        let ms_tag = Tag::new(ms_content).with_name(MS);

        Tag::new(TagList::new().with_tag(ms_tag.into()))
            .with_attribute(Attribute::RefId(ref_id_str.into()))
            .with_attribute(Attribute::N("PSThreadOptions".into()))
            .with_name(Obj)
    }
}

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct ApartmentState<'a> {
    #[builder(default = 3)]
    ref_id: usize,
    #[builder(default = "MTA")]
    to_string_value: &'a str,
    #[builder(default = 1)]
    int_value: i32,
}

impl<'a> Default for ApartmentState<'a> {
    fn default() -> Self {
        ApartmentState {
            ref_id: 3,
            to_string_value: "MTA",
            int_value: 1,
        }
    }
}

impl<'a> ApartmentState<'a> {
    pub fn mta() -> Self {
        Self::default()
    }

    pub fn to_tag(&self) -> Tag<'_, TagList<'_>, Obj> {
        let int_value_str = self.int_value.to_string();
        let ref_id_str = self.ref_id.to_string();
        
        let type_names = TagList::new()
            .with_tag(
                Tag::new(Text::from("System.Threading.ApartmentState"))
                    .with_name(T)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from("System.Enum"))
                    .with_name(T)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from("System.ValueType"))
                    .with_name(T)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from("System.Object"))
                    .with_name(T)
                    .into(),
            );

        let tn_tag = Tag::new(type_names)
            .with_attribute(Attribute::RefId("1".into()))
            .with_name(TN);

        let ms_content = TagList::new()
            .with_tag(tn_tag.into())
            .with_tag(
                Tag::new(Text::from(self.to_string_value))
                    .with_name(ToString)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from(int_value_str))
                    .with_name(I32)
                    .into(),
            );

        let ms_tag = Tag::new(ms_content).with_name(MS);

        Tag::new(TagList::new().with_tag(ms_tag.into()))
            .with_attribute(Attribute::RefId(ref_id_str.into()))
            .with_attribute(Attribute::N("ApartmentState".into()))
            .with_name(Obj)
    }
}

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct HostInfo {
    #[builder(default = 4)]
    ref_id: usize,
    #[builder(default = false)]
    is_host_null: bool,
    #[builder(default = false)]
    is_host_ui_null: bool,
    #[builder(default = false)]
    is_host_raw_ui_null: bool,
    #[builder(default = false)]
    use_runspace_host: bool,
}

impl HostInfo {
    pub fn to_tag(&self) -> Tag<'_, TagList<'_>, Obj> {
        let ref_id_str = self.ref_id.to_string();
        
        let ms_content = TagList::new()
            .with_tag(
                Tag::new(Text::from(if self.is_host_null { "true" } else { "false" }))
                    .with_attribute(Attribute::N("_isHostNull".into()))
                    .with_name(B)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from(if self.is_host_ui_null { "true" } else { "false" }))
                    .with_attribute(Attribute::N("_isHostUINull".into()))
                    .with_name(B)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from(if self.is_host_raw_ui_null { "true" } else { "false" }))
                    .with_attribute(Attribute::N("_isHostRawUINull".into()))
                    .with_name(B)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from(if self.use_runspace_host { "true" } else { "false" }))
                    .with_attribute(Attribute::N("_useRunspaceHost".into()))
                    .with_name(B)
                    .into(),
            );

        let ms_tag = Tag::new(ms_content).with_name(MS);

        Tag::new(TagList::new().with_tag(ms_tag.into()))
            .with_attribute(Attribute::RefId(ref_id_str.into()))
            .with_attribute(Attribute::N("HostInfo".into()))
            .with_name(Obj)
    }
}

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct InitRunspacepool<'a> {
    #[builder(default = 1)]
    ref_id: usize,
    min_runspaces: i32,
    max_runspaces: i32,
    #[builder(default)]
    ps_thread_options: PSThreadOptions<'a>,
    #[builder(default)]
    apartment_state: ApartmentState<'a>,
    #[builder(default, setter(strip_option))]
    host_info: Option<HostInfo>,
    #[builder(default = true)]
    application_arguments_null: bool,
}

impl<'a> InitRunspacepool<'a> {
    pub fn new(min_runspaces: i32, max_runspaces: i32) -> Self {
        InitRunspacepool {
            ref_id: 1,
            min_runspaces,
            max_runspaces,
            ps_thread_options: PSThreadOptions::default(),
            apartment_state: ApartmentState::default(),
            host_info: None,
            application_arguments_null: true,
        }
    }

    pub fn to_tag(&self) -> Tag<'_, TagList<'_>, Obj> {
        let min_runspaces_str = self.min_runspaces.to_string();
        let max_runspaces_str = self.max_runspaces.to_string();
        let ref_id_str = self.ref_id.to_string();
        
        let mut ms_content = TagList::new()
            .with_tag(
                Tag::new(Text::from(min_runspaces_str))
                    .with_attribute(Attribute::N("MinRunspaces".into()))
                    .with_name(I32)
                    .into(),
            )
            .with_tag(
                Tag::new(Text::from(max_runspaces_str))
                    .with_attribute(Attribute::N("MaxRunspaces".into()))
                    .with_name(I32)
                    .into(),
            )
            .with_tag(self.ps_thread_options.to_tag().into())
            .with_tag(self.apartment_state.to_tag().into());

        if let Some(host_info) = &self.host_info {
            ms_content = ms_content.with_tag(host_info.to_tag().into());
        }

        if self.application_arguments_null {
            ms_content = ms_content.with_tag(
                Tag::new(Text::from(""))
                    .with_attribute(Attribute::N("ApplicationArguments".into()))
                    .with_name(Nil)
                    .into(),
            );
        }

        let ms_tag = Tag::new(ms_content).with_name(MS);

        Tag::new(TagList::new().with_tag(ms_tag.into()))
            .with_attribute(Attribute::RefId(ref_id_str.into()))
            .with_name(Obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_runspacepool_basic() {
        let init_runspacepool = InitRunspacepool::new(1, 1);
        let tag = init_runspacepool.to_tag();

        // Test that the tag creation works
        let element = tag.into_element();
        let xml_string = format!("{}", element);
        
        // Basic assertions
        assert!(xml_string.contains("MinRunspaces"));
        assert!(xml_string.contains("MaxRunspaces"));
        
        println!("Generated XML:\n{}", xml_string);
    }

    #[test]
    fn test_ps_thread_options() {
        let ps_thread_options = PSThreadOptions::default();
        let tag = ps_thread_options.to_tag();

        let element = tag.into_element();
        let xml_string = format!("{}", element);
        
        assert!(xml_string.contains("System.Management.Automation.Runspaces.PSThreadOptions"));
        assert!(xml_string.contains("Default"));
        
        println!("PSThreadOptions XML:\n{}", xml_string);
    }

    #[test]
    fn test_apartment_state() {
        let apartment_state = ApartmentState::mta();
        let tag = apartment_state.to_tag();

        let element = tag.into_element();
        let xml_string = format!("{}", element);
        
        assert!(xml_string.contains("System.Threading.ApartmentState"));
        assert!(xml_string.contains("MTA"));
        
        println!("ApartmentState XML:\n{}", xml_string);
    }

    #[test]
    fn test_with_host_info() {
        let host_info = HostInfo::builder()
            .ref_id(4)
            .is_host_null(false)
            .is_host_ui_null(false)
            .is_host_raw_ui_null(false)
            .use_runspace_host(false)
            .build();

        let init_runspacepool = InitRunspacepool::builder()
            .ref_id(1)
            .min_runspaces(1)
            .max_runspaces(1)
            .ps_thread_options(PSThreadOptions::default())
            .apartment_state(ApartmentState::default())
            .host_info(host_info)
            .build();

        let tag = init_runspacepool.to_tag();
        let element = tag.into_element();
        let xml_string = format!("{}", element);
        
        assert!(xml_string.contains("HostInfo"));
        assert!(xml_string.contains("_isHostNull"));
        
        println!("InitRunspacepool with HostInfo XML:\n{}", xml_string);
    }
}
