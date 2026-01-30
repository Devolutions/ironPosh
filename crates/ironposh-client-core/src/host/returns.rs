use std::{borrow::Cow, collections::BTreeMap, collections::HashMap};

use super::{methods, traits::ToPs};
use ironposh_psrp::{
    ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsProperty, PsType, PsValue,
};

fn obj_with_extended_props(type_names: &[&'static str], props: Vec<(&str, PsValue)>) -> PsValue {
    let mut extended_properties = BTreeMap::new();
    for (name, value) in props {
        extended_properties.insert(
            name.to_string(),
            PsProperty {
                name: name.to_string(),
                value,
            },
        );
    }

    PsValue::Object(ComplexObject {
        type_def: Some(PsType {
            type_names: type_names.iter().map(|s| Cow::Borrowed(*s)).collect(),
        }),
        to_string: None,
        content: ComplexObjectContent::Standard,
        adapted_properties: BTreeMap::new(),
        extended_properties,
    })
}

impl<S: ::std::hash::BuildHasher> ToPs for HashMap<String, PsValue, S> {
    fn to_ps(v: Self) -> Option<PsValue> {
        // Represent as a Hashtable. (Some WS-Man endpoints reject Prompt responses when the
        // payload is typed as `PSPrimitiveDictionary`, but accept plain `Hashtable`.)
        let mut dict = BTreeMap::new();
        for (k, vv) in v {
            dict.insert(PsValue::Primitive(PsPrimitiveValue::Str(k)), vv);
        }
        Some(PsValue::Object(ComplexObject {
            type_def: Some(PsType {
                type_names: vec![
                    Cow::Borrowed("System.Collections.Hashtable"),
                    Cow::Borrowed("System.Object"),
                ],
            }),
            to_string: None,
            content: ComplexObjectContent::Container(Container::Dictionary(dict)),
            adapted_properties: BTreeMap::new(),
            extended_properties: BTreeMap::new(),
        }))
    }
}

impl ToPs for methods::PSCredential {
    fn to_ps(v: Self) -> Option<PsValue> {
        // Best-effort PSCredential representation: include both PascalCase and camelCase names,
        // since different remoting stacks may look for either.
        let password = PsValue::Primitive(PsPrimitiveValue::SecureString(v.password));
        Some(obj_with_extended_props(
            &["System.Management.Automation.PSCredential", "System.Object"],
            vec![
                ("UserName", PsValue::from(v.user_name.clone())),
                ("userName", PsValue::from(v.user_name)),
                ("Password", password.clone()),
                ("password", password),
            ],
        ))
    }
}

impl ToPs for Vec<i32> {
    fn to_ps(v: Self) -> Option<PsValue> {
        let values: Vec<PsValue> = v.into_iter().map(PsValue::from).collect();
        Some(PsValue::from_array(values))
    }
}

impl ToPs for methods::KeyInfo {
    fn to_ps(v: Self) -> Option<PsValue> {
        Some(obj_with_extended_props(
            &[
                "System.Management.Automation.Host.KeyInfo",
                "System.ValueType",
                "System.Object",
            ],
            vec![
                ("virtualKeyCode", PsValue::from(v.virtual_key_code)),
                (
                    "character",
                    PsValue::Primitive(PsPrimitiveValue::Char(v.character)),
                ),
                ("controlKeyState", PsValue::from(v.control_key_state)),
                ("keyDown", PsValue::from(v.key_down)),
                // also provide PascalCase as seen in PowerShell property names
                ("VirtualKeyCode", PsValue::from(v.virtual_key_code)),
                (
                    "Character",
                    PsValue::Primitive(PsPrimitiveValue::Char(v.character)),
                ),
                ("ControlKeyState", PsValue::from(v.control_key_state)),
                ("KeyDown", PsValue::from(v.key_down)),
            ],
        ))
    }
}

impl ToPs for Vec<Vec<methods::BufferCell>> {
    fn to_ps(v: Self) -> Option<PsValue> {
        // Best-effort: represent 2D array as ArrayList of ArrayList of BufferCell objects.
        // This matches how many PSRP stacks encode multi-dimensional data in practice.
        let rows: Vec<PsValue> = v
            .into_iter()
            .map(|row| {
                let cells: Vec<PsValue> = row
                    .into_iter()
                    .map(|c| {
                        obj_with_extended_props(
                            &[
                                "System.Management.Automation.Host.BufferCell",
                                "System.ValueType",
                                "System.Object",
                            ],
                            vec![
                                (
                                    "character",
                                    PsValue::Primitive(PsPrimitiveValue::Char(c.character)),
                                ),
                                ("foregroundColor", PsValue::from(c.foreground)),
                                ("backgroundColor", PsValue::from(c.background)),
                                ("bufferCellType", PsValue::from(c.flags)),
                            ],
                        )
                    })
                    .collect();
                PsValue::from_array(cells)
            })
            .collect();
        Some(PsValue::from_array(rows))
    }
}

impl ToPs for methods::Coordinates {
    fn to_ps(v: Self) -> Option<PsValue> {
        Some(obj_with_extended_props(
            &[
                "System.Management.Automation.Host.Coordinates",
                "System.ValueType",
                "System.Object",
            ],
            vec![
                ("x", PsValue::from(v.x)),
                ("y", PsValue::from(v.y)),
                ("X", PsValue::from(v.x)),
                ("Y", PsValue::from(v.y)),
            ],
        ))
    }
}

impl ToPs for methods::Size {
    fn to_ps(v: Self) -> Option<PsValue> {
        Some(obj_with_extended_props(
            &[
                "System.Management.Automation.Host.Size",
                "System.ValueType",
                "System.Object",
            ],
            vec![
                ("width", PsValue::from(v.width)),
                ("height", PsValue::from(v.height)),
                ("Width", PsValue::from(v.width)),
                ("Height", PsValue::from(v.height)),
            ],
        ))
    }
}
