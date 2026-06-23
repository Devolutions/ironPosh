use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use super::{methods, traits::ToPs};
use ironposh_psrp::ps_value::ToPsValue;
use ironposh_psrp::{
    ComplexObject, ComplexObjectContent, Container, Properties, PsPrimitiveValue, PsType, PsValue,
};

/// Return types whose CLIXML is fully macro-derived (`ToPsValue`); `ToPs` here
/// is just the thin positional-return adapter the host dispatch calls.
macro_rules! to_ps_via_derive {
    ($($t:ty),* $(,)?) => {
        $(
            impl ToPs for $t {
                fn to_ps(v: Self) -> Option<PsValue> {
                    Some(v.to_ps_value())
                }
            }
        )*
    };
}

to_ps_via_derive!(
    methods::Coordinates,
    methods::Size,
    methods::KeyInfo,
    methods::PSCredential,
    methods::BufferCell,
    Vec<i32>,
    Vec<Vec<methods::BufferCell>>,
);

impl<S: ::std::hash::BuildHasher> ToPs for HashMap<String, PsValue, S> {
    fn to_ps(v: Self) -> Option<PsValue> {
        // A genuinely dynamic dictionary (Prompt result: field names known only
        // at runtime), so it stays a hand-built Hashtable rather than a derived
        // struct. (Some WS-Man endpoints reject `PSPrimitiveDictionary` for
        // Prompt responses but accept a plain `Hashtable`.)
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
            properties: Properties::new(),
        }))
    }
}
