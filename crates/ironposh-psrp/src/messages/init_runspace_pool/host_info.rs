use super::HostDefaultData;
use ironposh_macros::{PsDeserialize, PsSerialize};

/// HOST_INFO (MS-PSRP §2.2.3.14): the client host description sent with
/// runspace-pool and pipeline creation.
///
/// Fully macro-derived. The four `_isHost*`/`_useRunspaceHost` flags are plain
/// `<B>` properties; `host_default_data` nests one level under a `data` member
/// (`#[ps(wrap = "data")]`) into the integer-keyed value dictionary derived on
/// [`HostDefaultData`]. Missing flags default to `false` and a missing
/// `_hostDefaultData` falls back to [`HostDefaultData::default`].
#[expect(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
pub struct HostInfo {
    #[builder(default = false)]
    #[ps(name = "_isHostNull", default)]
    pub is_host_null: bool,
    #[builder(default = false)]
    #[ps(name = "_isHostUINull", default)]
    pub is_host_ui_null: bool,
    #[builder(default = false)]
    #[ps(name = "_isHostRawUINull", default)]
    pub is_host_raw_ui_null: bool,
    #[builder(default = false)]
    #[ps(name = "_useRunspaceHost", default)]
    pub use_runspace_host: bool,
    #[ps(name = "_hostDefaultData", wrap = "data", default)]
    pub host_default_data: HostDefaultData,
}

impl HostInfo {
    pub fn enabled_all(host_data: HostDefaultData) -> Self {
        Self {
            is_host_null: true,
            is_host_ui_null: true,
            is_host_raw_ui_null: true,
            use_runspace_host: true,
            host_default_data: host_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::init_runspace_pool::{Coordinates, Size};
    use crate::ps_value::ComplexObject;

    #[test]
    fn test_host_info_serialization_deserialization() {
        let original_host_info = HostInfo {
            is_host_null: false,
            is_host_ui_null: false,
            is_host_raw_ui_null: false,
            use_runspace_host: true,
            host_default_data: HostDefaultData {
                foreground_color: 7,
                background_color: 0,
                cursor_position: Coordinates { x: 0, y: 0 },
                window_position: Coordinates { x: 0, y: 0 },
                cursor_size: 25,
                window_size: Size {
                    width: 120,
                    height: 50,
                },
                buffer_size: Size {
                    width: 120,
                    height: 3000,
                },
                max_window_size: Size {
                    width: 120,
                    height: 50,
                },
                max_physical_window_size: Size {
                    width: 120,
                    height: 50,
                },
                window_title: "PowerShell".to_string(),
                locale: "en-US".to_string(),
                ui_locale: "en-US".to_string(),
            },
        };

        let complex_object: ComplexObject = original_host_info.clone().into();
        let deserialized_host_info = HostInfo::try_from(complex_object).unwrap();

        assert_eq!(original_host_info, deserialized_host_info);
    }
}
