//! Tests for the typed value-access layer (RFC #12, L0/L1):
//! `FromPsValue`/`ToPsValue`, the `req`/`opt` accessors, the
//! `ComplexObject` builder, and the `as_string_array` fix.

use crate::ps_value::{ComplexObject, ComplexObjectContent, Container, PsPrimitiveValue, PsValue};

#[test]
fn builder_writes_property_name_once_and_roundtrips() {
    let obj = ComplexObject::standard()
        .extended("MinRunspaces", 1i32)
        .extended("MaxRunspaces", 4i32)
        .build();

    // The value is stored under its name with no duplicated name field.
    assert!(obj.properties.get("MinRunspaces").is_some());
    assert_eq!(obj.properties.extended().count(), 2);

    assert_eq!(obj.req::<i32>("MinRunspaces").unwrap(), 1);
    assert_eq!(obj.req::<i32>("MaxRunspaces").unwrap(), 4);
}

#[test]
fn req_missing_property_reports_name() {
    let obj = ComplexObject::standard().build();
    let err = obj.req::<i32>("MinRunspaces").unwrap_err().to_string();
    assert!(
        err.contains("MinRunspaces"),
        "error should name the property: {err}"
    );
}

#[test]
fn req_type_mismatch_reports_name_and_expected_type() {
    let obj = ComplexObject::standard()
        .extended("Count", "not-a-number")
        .build();
    let err = obj.req::<i32>("Count").unwrap_err().to_string();
    assert!(
        err.contains("Count"),
        "error should name the property: {err}"
    );
    assert!(
        err.contains("I32"),
        "error should name expected type: {err}"
    );
}

#[test]
fn opt_missing_and_nil_are_none_present_is_some() {
    let present = ComplexObject::standard().extended("X", 7i32).build();
    assert_eq!(present.opt::<i32>("X").unwrap(), Some(7));

    let missing = ComplexObject::standard().build();
    assert_eq!(missing.opt::<i32>("X").unwrap(), None);

    let nil = ComplexObject::standard().extended("X", None::<i32>).build();
    assert_eq!(nil.opt::<i32>("X").unwrap(), None);
}

#[test]
fn get_property_searches_adapted_and_extended() {
    let obj = ComplexObject::standard()
        .adapted("A", 1i32)
        .extended("E", 2i32)
        .build();
    assert_eq!(obj.req::<i32>("A").unwrap(), 1);
    assert_eq!(obj.req::<i32>("E").unwrap(), 2);
}

#[test]
fn from_ps_value_for_vec_reads_lists() {
    let value = PsValue::from_string_array(vec!["a".into(), "b".into()]);
    let strings = Vec::<String>::from_ps_value(&value).unwrap();
    assert_eq!(strings, vec!["a".to_string(), "b".to_string()]);
}

// Bring the trait method into scope for the test above.
use crate::ps_value::FromPsValue;

#[test]
fn as_string_array_reads_container() {
    let value = PsValue::from_string_array(vec!["x".into(), "y".into()]);
    assert_eq!(
        value.as_string_array(),
        Some(vec!["x".to_string(), "y".to_string()])
    );
}

#[test]
fn as_string_array_rejects_non_string_elements() {
    let value = PsValue::from_array(vec![
        PsValue::Primitive(PsPrimitiveValue::Str("ok".into())),
        PsValue::Primitive(PsPrimitiveValue::I32(3)),
    ]);
    // Previously the stub returned Some(vec![]); now a non-string element is rejected.
    assert_eq!(value.as_string_array(), None);
}

#[test]
fn as_string_array_rejects_primitives() {
    assert_eq!(PsValue::from(42i32).as_string_array(), None);
}

#[test]
fn to_ps_value_option_none_is_nil() {
    use crate::ps_value::ToPsValue;
    assert_eq!(
        None::<i32>.to_ps_value(),
        PsValue::Primitive(PsPrimitiveValue::Nil)
    );
    assert_eq!(Some(5i32).to_ps_value(), PsValue::from(5i32));
}

#[test]
fn builder_container_content_and_types() {
    let obj = ComplexObject::builder(ComplexObjectContent::Container(Container::List(vec![
        PsValue::from(1i32),
    ])))
    .type_names([std::borrow::Cow::Borrowed("System.Collections.ArrayList")])
    .to_string_repr("1")
    .build();
    assert!(matches!(obj.content, ComplexObjectContent::Container(_)));
    assert_eq!(obj.to_string.as_deref(), Some("1"));
    assert!(obj.type_def.is_some());
}
