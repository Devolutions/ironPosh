use super::{PsObject, PsProperty, PsValue};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

use xml::builder::{Attribute, Element};

/// ------------------------------------------------------------------------------------------------
/// 1.  PsValue → <xml> element
/// ------------------------------------------------------------------------------------------------
impl<'a> PsValue {
    pub fn to_element(&'a self) -> Element<'a> {
        match self {
            PsValue::Str(s) => Element::new("S").set_text_owned(s.clone()),
            PsValue::Bool(b) => Element::new("B").set_text_owned(b.to_string()),
            PsValue::I32(i) => Element::new("I32").set_text_owned(i.to_string()),
            PsValue::U32(u) => Element::new("U32").set_text_owned(u.to_string()),
            PsValue::I64(i) => Element::new("I64").set_text_owned(i.to_string()),
            PsValue::Guid(g) => Element::new("G").set_text_owned(g.clone()),
            PsValue::Nil => Element::new("Nil"), // empty tag
            PsValue::Bytes(b) => Element::new("BA").set_text_owned(B64.encode(b)),
            PsValue::Version(v) => Element::new("Version").set_text_owned(v.clone()),
            PsValue::Object(o) => o.to_element(), // recursive
        }
    }
}

/// ------------------------------------------------------------------------------------------------
/// 2.  PsProperty → <xml> element
/// ------------------------------------------------------------------------------------------------
impl<'a> PsProperty {
    pub fn to_element(&'a self) -> Element<'a> {
        // Convert the inner PsValue first
        let mut elem = self.value.to_element();

        // Only <Obj> needs its name on *wrapper* element, not inside.
        // For primitives we add an attribute directly.
        if let Some(name) = &self.name {
            elem = elem.add_attribute(Attribute::new("N", name.clone()));
        }
        if let Some(ref_id) = self.ref_id {
            elem = elem.add_attribute(Attribute::new("RefId", ref_id.to_string()));
        }

        elem
    }
}

/// ------------------------------------------------------------------------------------------------
/// 3.  Full PsObject → <Obj>
/// ------------------------------------------------------------------------------------------------
impl<'a> PsObject {
    pub fn to_element(&'a self) -> Element<'a> {
        // ---------- <Obj> ---------- //
        let mut obj_el = Element::new("Obj");
        obj_el = obj_el.add_attribute(Attribute::new("RefId", self.ref_id.to_string()));

        // ---------- <TN> or <TNRef> ---------- //
        if let Some(tn_ref) = self.tn_ref {
            obj_el = obj_el.add_child(
                Element::new("TNRef").add_attribute(Attribute::new("RefId", tn_ref.to_string())),
            );
        } else if let Some(type_names) = &self.type_names {
            //   <TN RefId="0"><T>..</T><T>..</T></TN>
            let tn = type_names.iter().fold(
                Element::new("TN").add_attribute(Attribute::new("RefId", "0")),
                |acc, t| acc.add_child(Element::new("T").set_text_owned(t.clone())),
            );
            obj_el = obj_el.add_child(tn);
        }

        // ---------- <ToString> for enums ---------- //
        if let Some(to_string) = &self.to_string {
            obj_el = obj_el.add_child(Element::new("ToString").set_text_owned(to_string.clone()));
        }

        // ---------- Direct enum value ---------- //
        if let Some(enum_value) = self.enum_value {
            obj_el = obj_el.add_child(Element::new("I32").set_text_owned(enum_value.to_string()));
        }

        // ---------- containers ---------- //
        if !self.ms.is_empty() {
            let ms = self
                .ms
                .iter()
                .fold(Element::new("MS"), |acc, p| acc.add_child(p.to_element()));
            obj_el = obj_el.add_child(ms);
        }

        if !self.props.is_empty() {
            let pr = self.props.iter().fold(Element::new("Props"), |acc, p| {
                acc.add_child(p.to_element())
            });
            obj_el = obj_el.add_child(pr);
        }

        if !self.lst.is_empty() {
            let lst = self
                .lst
                .iter()
                .fold(Element::new("LST"), |acc, p| acc.add_child(p.to_element()));
            obj_el = obj_el.add_child(lst);
        }

        if !self.dct.is_empty() {
            // <DCT><En><Key/><Value/></En>…</DCT>
            let entries: Vec<_> = self
                .dct
                .iter()
                .map(|(k, v)| {
                    Element::new("En")
                        .add_child(k.to_element().add_attribute(Attribute::new("N", "Key")))
                        .add_child(v.to_element().add_attribute(Attribute::new("N", "Value")))
                })
                .collect();

            let dct = Element::new("DCT").add_children(entries);
            obj_el = obj_el.add_child(dct);
        }

        obj_el
    }
}
