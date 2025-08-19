use std::collections::{BTreeMap, HashMap};

use crate::{ComplexObject, ComplexObjectContent, Container, PsType, PsValue};

use super::{PsPrimitiveValue, PsProperty};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

use tracing::{debug, trace};
use xml::builder::{Attribute, Element};

type Result<T> = std::result::Result<T, crate::PowerShellRemotingError>;

#[derive(Debug, Default)]
pub struct RefIdMap<'a, T> {
    pub map: HashMap<&'a T, u32>,
    pub next_id: u32,
}

impl<'a, T> RefIdMap<'a, T> {
    pub fn new() -> Self {
        RefIdMap {
            map: HashMap::new(),
            next_id: 0,
        }
    }
}

impl<'a, T> RefIdMap<'a, T>
where
    T: std::hash::Hash + Eq,
{
    pub fn contains(&self, item: &T) -> bool {
        self.map.contains_key(item)
    }

    pub fn insert_new(&mut self, item: &'a T) -> Result<u32> {
        if let Some(existing_id) = self.map.get(item) {
            trace!("Item already exists in RefIdMap with id={}", existing_id);
            Err(crate::PowerShellRemotingError::SerializationError(
                "Attempted to insert duplicate item into RefIdMap",
            ))
        } else {
            let id = self.next_id;
            trace!("Assigning new RefId={} to item", id);
            self.map.insert(item, id);
            self.next_id += 1;
            Ok(id)
        }
    }
}
/// ------------------------------------------------------------------------------------------------
/// 1.  PsValue → <xml> element
/// ------------------------------------------------------------------------------------------------
impl<'a> PsPrimitiveValue {
    pub fn to_element(&'a self) -> Result<Element<'a>> {
        Ok(match self {
            PsPrimitiveValue::Str(s) => Element::new("S").set_text_owned(s.clone()),
            PsPrimitiveValue::Bool(b) => Element::new("B").set_text_owned(b.to_string()),
            PsPrimitiveValue::I32(i) => Element::new("I32").set_text_owned(i.to_string()),
            PsPrimitiveValue::U32(u) => Element::new("U32").set_text_owned(u.to_string()),
            PsPrimitiveValue::I64(i) => Element::new("I64").set_text_owned(i.to_string()),
            PsPrimitiveValue::U64(u) => Element::new("U64").set_text_owned(u.to_string()),
            PsPrimitiveValue::Guid(g) => Element::new("G").set_text_owned(g.clone()),
            PsPrimitiveValue::Nil => Element::new("Nil"), // empty tag
            PsPrimitiveValue::Bytes(b) => Element::new("BA").set_text_owned(B64.encode(b)),
            PsPrimitiveValue::Version(v) => Element::new("Version").set_text_owned(v.clone()),
            PsPrimitiveValue::DateTime(dt) => Element::new("DT").set_text_owned(dt.clone()),
        })
    }
}

impl<'a> PsValue {
    pub fn to_element_as_root(&'a self) -> Result<Element<'a>> {
        let mut objects_map = RefIdMap::new();
        let mut types_map = RefIdMap::new();
        self.to_element(&mut objects_map, &mut types_map)
    }

    pub fn to_element(
        &'a self,
        objects_map: &mut RefIdMap<'a, ComplexObject>,
        types_map: &mut RefIdMap<'a, PsType>,
    ) -> Result<Element<'a>> {
        match self {
            PsValue::Primitive(ps_primitive_value) => Ok(ps_primitive_value.to_element()?),
            PsValue::Object(complex_object) => complex_object.to_element(objects_map, types_map),
        }
    }
}

impl<'a> PsProperty {
    pub fn to_element(
        &'a self,
        objects_map: &mut RefIdMap<'a, ComplexObject>,
        types_map: &mut RefIdMap<'a, PsType>,
    ) -> Result<Element<'a>> {
        Ok(self
            .value
            .to_element(objects_map, types_map)?
            .add_attribute(Attribute::new("N", &self.name)))
    }
}

impl<'a> PsType {
    pub fn to_element(&'a self, type_maps: &mut RefIdMap<'a, PsType>) -> Result<Element<'a>> {
        if type_maps.contains(self) {
            // If this type has already been serialized, return a reference element.
            let ref_id = type_maps.map.get(self).unwrap();
            debug!("Creating TNRef for existing type with RefId={}", ref_id);
            return Ok(
                Element::new("TNRef").add_attribute(Attribute::new("RefId", ref_id.to_string()))
            );
        }

        let ref_id = type_maps.insert_new(self)?;
        trace!(
            "Creating TN with new RefId={} and {} type names",
            ref_id,
            self.type_names.len()
        );
        trace!(?self.type_names, "Type names for RefId={}", ref_id);
        let mut element =
            Element::new("TN").add_attribute(Attribute::new("RefId", ref_id.to_string()));
        for type_name in &self.type_names {
            element = element.add_child(Element::new("T").set_text_owned(type_name.to_string()));
        }
        Ok(element)
    }
}

impl<'a> Container {
    pub fn to_element(
        &'a self,
        objects_map: &mut RefIdMap<'a, ComplexObject>,
        types_map: &mut RefIdMap<'a, PsType>,
    ) -> Result<Element<'a>> {
        Ok(match self {
            // Stacks, Queues, and Lists all serialize to an <LST> tag.
            // The <TN> in the parent <Obj> is what differentiates their type.
            Container::Stack(values) => {
                let mut element = Element::new("STK");
                for value in values {
                    element = element.add_child(value.to_element(objects_map, types_map)?);
                }
                element
            }
            Container::Queue(values) => {
                let mut element = Element::new("QUE");
                for value in values {
                    element = element.add_child(value.to_element(objects_map, types_map)?);
                }
                element
            }
            Container::List(values) => {
                let mut element = Element::new("LST");
                for value in values {
                    element = element.add_child(value.to_element(objects_map, types_map)?);
                }
                element
            }
            // Dictionaries serialize to a <DCT> tag with <En> entries.
            Container::Dictionary(map) => {
                let mut element = Element::new("DCT");
                for (key, value) in map {
                    let key_element = key
                        .to_element(objects_map, types_map)?
                        .add_attribute(Attribute::new("N", "Key"));
                    let value_element = value
                        .to_element(objects_map, types_map)?
                        .add_attribute(Attribute::new("N", "Value"));

                    let entry_element = Element::new("En")
                        .add_child(key_element)
                        .add_child(value_element);

                    element = element.add_child(entry_element);
                }
                element
            }
        })
    }
}

impl<'a> ComplexObject {
    pub fn to_element_as_root(&'a self) -> Result<Element<'a>> {
        let mut objects_map = RefIdMap::new();
        let mut types_map = RefIdMap::new();
        self.to_element(&mut objects_map, &mut types_map)
    }

    pub fn to_element(
        &'a self,
        objects_map: &mut RefIdMap<'a, ComplexObject>,
        types_map: &mut RefIdMap<'a, PsType>,
    ) -> Result<Element<'a>> {
        let ref_id = if let Some(obj_ref_id) = objects_map.map.get(self) {
            // If this object has already been serialized, return a reference element.
            trace!("Creating Ref for existing object with RefId={}", obj_ref_id);
            return Ok(
                Element::new("Ref").add_attribute(Attribute::new("RefId", obj_ref_id.to_string()))
            );
        } else {
            // Assign a new RefId to this object and store it in the map.
            let new_ref_id = objects_map.insert_new(self)?;
            trace!("Creating Obj with new RefId={}", new_ref_id);
            new_ref_id
        };

        // 1. Create the root <Obj> element and add its RefId
        let mut element =
            Element::new("Obj").add_attribute(Attribute::new("RefId", ref_id.to_string()));

        // 2. Add optional metadata: <TN> and <ToString>
        if let Some(type_def) = &self.type_def {
            element = element.add_child(type_def.to_element(types_map)?);
        }
        if let Some(to_string) = &self.to_string {
            element = element.add_child(Element::new("ToString").set_text(to_string.as_str()));
        }

        // 3. Add the primary content based on its type
        match &self.content {
            ComplexObjectContent::ExtendedPrimitive(value) => {
                element = element.add_child(value.to_element()?);
            }
            ComplexObjectContent::Container(container) => {
                element = element.add_child(container.to_element(objects_map, types_map)?);
            }
            ComplexObjectContent::PsEnums(ps_enum) => {
                // For enums, the "content" is the <ToString> and <I32> tags.
                // Note: The general <ToString> is added above; the spec can be
                // interpreted in different ways, but often an enum's specific
                // name is placed in the general <ToString> tag.
                // We will add the required integer value here.
                element = element
                    .add_child(Element::new("I32").set_text_owned(ps_enum.value.to_string()));
            }
            ComplexObjectContent::Standard => {
                // A standard object's content is defined solely by its properties (<MS>).
                // No extra content element is needed here.
            }
        }

        // 4. Add Adapted Properties (<Props>) if they exist
        if !self.adapted_properties.is_empty() {
            let mut props_element = Element::new("Props");
            for prop in self.adapted_properties.values() {
                props_element = props_element.add_child(prop.to_element(objects_map, types_map)?);
            }
            element = element.add_child(props_element);
        }

        // 5. Add Extended/Standard Properties (<MS>) if they exist
        if !self.extended_properties.is_empty() {
            let mut ms_element = Element::new("MS");
            for prop in self.extended_properties.values() {
                ms_element = ms_element.add_child(prop.to_element(objects_map, types_map)?);
            }
            element = element.add_child(ms_element);
        }

        Ok(element)
    }
}
