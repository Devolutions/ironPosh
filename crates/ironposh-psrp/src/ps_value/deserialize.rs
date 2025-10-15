use super::{
    ComplexObject, ComplexObjectContent, Container, PsEnums, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use ironposh_xml::parser::{XmlDeserialize, XmlVisitor};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use tracing::trace;

type Result<T> = std::result::Result<T, ironposh_xml::XmlError>;

/// A visitor for XML deserialization.
///
/// Kept for backward compatibility as primitives don't need context.
#[derive(Default)]
pub struct PsPrimitiveValueVisitor<'a> {
    value: Option<PsPrimitiveValue>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl PsPrimitiveValueVisitor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> XmlVisitor<'a> for PsPrimitiveValueVisitor<'a> {
    type Value = PsPrimitiveValue;

    fn visit_node(&mut self, node: ironposh_xml::parser::Node<'a, 'a>) -> Result<()> {
        if !node.is_element() {
            return Ok(());
        }

        let tag_name = node.tag_name().name();

        match tag_name {
            "S" => {
                let text = node.text().unwrap_or("").to_string();
                self.value = Some(PsPrimitiveValue::Str(text));
            }
            "B" => {
                let text = node.text().unwrap_or("false");
                let bool_val = text.parse::<bool>().map_err(|_| {
                    ironposh_xml::XmlError::GenericError(format!("Invalid boolean value: {text}"))
                })?;
                self.value = Some(PsPrimitiveValue::Bool(bool_val));
            }
            "I32" => {
                let text = node.text().unwrap_or("0");
                let int_val = text.parse::<i32>().map_err(|_| {
                    ironposh_xml::XmlError::GenericError(format!("Invalid i32 value: {text}"))
                })?;
                self.value = Some(PsPrimitiveValue::I32(int_val));
            }
            "U32" => {
                let text = node.text().unwrap_or("0");
                let uint_val = text.parse::<u32>().map_err(|_| {
                    ironposh_xml::XmlError::GenericError(format!("Invalid u32 value: {text}"))
                })?;
                self.value = Some(PsPrimitiveValue::U32(uint_val));
            }
            "U64" => {
                let text = node.text().unwrap_or("0");
                let uint_val = text.parse::<u64>().map_err(|_| {
                    ironposh_xml::XmlError::GenericError(format!("Invalid u64 value: {text}"))
                })?;
                self.value = Some(PsPrimitiveValue::U64(uint_val));
            }
            "I64" => {
                let text = node.text().unwrap_or("0");
                let long_val = text.parse::<i64>().map_err(|_| {
                    ironposh_xml::XmlError::GenericError(format!("Invalid i64 value: {text}"))
                })?;
                self.value = Some(PsPrimitiveValue::I64(long_val));
            }
            "DT" => {
                let text = node.text().unwrap_or("").to_string();
                self.value = Some(PsPrimitiveValue::DateTime(text));
            }
            "G" => {
                let text = node.text().unwrap_or("").to_string();
                self.value = Some(PsPrimitiveValue::Guid(text));
            }
            "C" => {
                let text = node.text().unwrap_or("0");
                let char_code = text.parse::<u32>().map_err(|_| {
                    ironposh_xml::XmlError::GenericError(format!("Invalid character code: {text}"))
                })?;
                let char_val = char::from_u32(char_code).ok_or_else(|| {
                    ironposh_xml::XmlError::GenericError(format!(
                        "Invalid Unicode character code: {char_code}"
                    ))
                })?;
                self.value = Some(PsPrimitiveValue::Char(char_val));
            }
            "Nil" => {
                self.value = Some(PsPrimitiveValue::Nil);
            }
            "BA" => {
                let text = node.text().unwrap_or("");
                let bytes = B64.decode(text).map_err(|_| {
                    ironposh_xml::XmlError::GenericError(format!("Invalid base64 data: {text}"))
                })?;
                self.value = Some(PsPrimitiveValue::Bytes(bytes));
            }
            "Version" => {
                let text = node.text().unwrap_or("").to_string();
                self.value = Some(PsPrimitiveValue::Version(text));
            }
            _ => {
                return Err(ironposh_xml::XmlError::UnexpectedTag(tag_name.to_string()));
            }
        }

        Ok(())
    }

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
    ) -> Result<()> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Value> {
        self.value.ok_or_else(|| {
            ironposh_xml::XmlError::GenericError("No PsPrimitiveValue found".to_string())
        })
    }
}

impl<'a> XmlDeserialize<'a> for PsPrimitiveValue {
    type Visitor = PsPrimitiveValueVisitor<'a>;

    fn visitor() -> Self::Visitor {
        PsPrimitiveValueVisitor::new()
    }
}

/// Context for deserialization that maintains reference maps.
#[derive(Debug, Default)]
pub struct DeserializationContext {
    /// Maps RefId to PsType for type references (<TNRef RefId="...">)
    pub type_refs: HashMap<String, PsType>,
    /// Maps RefId to ComplexObject for object references (<Ref RefId="...">)
    pub object_refs: HashMap<String, ComplexObject>,
}

impl DeserializationContext {
    pub fn new() -> Self {
        Self {
            type_refs: HashMap::new(),
            object_refs: HashMap::new(),
        }
    }

    pub fn register_type(&mut self, ref_id: String, ps_type: PsType) {
        trace!(
            "Registering type reference RefId={} with {} type names",
            ref_id,
            ps_type.type_names.len()
        );
        trace!(?ps_type, "Type details for RefId={}", ref_id);
        self.type_refs.insert(ref_id, ps_type);
    }

    pub fn get_type(&self, ref_id: &str) -> Option<&PsType> {
        let result = self.type_refs.get(ref_id);
        trace!(
            "Looking up type reference RefId={}, found={}",
            ref_id,
            result.is_some()
        );
        result
    }

    pub fn register_object(&mut self, ref_id: String, object: ComplexObject) {
        trace!(?object, "Object details for RefId={}", ref_id);
        self.object_refs.insert(ref_id, object);
    }

    pub fn get_object(&self, ref_id: &str) -> Option<&ComplexObject> {
        let result = self.object_refs.get(ref_id);
        trace!(
            "Looking up object reference RefId={}, found={}",
            ref_id,
            result.is_some()
        );
        if result.is_none() {
            trace!(
                "Available object RefIds: {:?}",
                self.object_refs.keys().collect::<Vec<_>>()
            );
        }
        result
    }
}

/// Context-aware visitor trait for deserialization with reference resolution
pub trait PsXmlVisitor<'a> {
    type Value;

    fn visit_node(
        &mut self,
        node: ironposh_xml::parser::Node<'a, 'a>,
        context: &mut DeserializationContext,
    ) -> Result<()>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
        context: &mut DeserializationContext,
    ) -> Result<()>;

    fn finish(self) -> Result<Self::Value>;
}

/// Context-aware deserialize trait
pub trait PsXmlDeserialize<'a>: Sized {
    type Visitor: PsXmlVisitor<'a, Value = Self>;

    fn visitor_with_context() -> Self::Visitor;

    fn from_node_with_context(
        node: ironposh_xml::parser::Node<'a, 'a>,
        context: &mut DeserializationContext,
    ) -> Result<Self> {
        let mut visitor = Self::visitor_with_context();
        visitor.visit_node(node, context)?;
        visitor.finish()
    }

    fn from_children_with_context(
        children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
        context: &mut DeserializationContext,
    ) -> Result<Self> {
        let mut visitor = Self::visitor_with_context();
        visitor.visit_children(children, context)?;
        visitor.finish()
    }
}

/// Context-aware PsType visitor that handles type references
#[derive(Default)]
pub struct PsTypeContextVisitor<'a> {
    type_names: Vec<Cow<'static, str>>,
    resolved_type: Option<PsType>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl PsTypeContextVisitor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> PsXmlVisitor<'a> for PsTypeContextVisitor<'a> {
    type Value = PsType;

    fn visit_node(
        &mut self,
        node: ironposh_xml::parser::Node<'a, 'a>,
        context: &mut DeserializationContext,
    ) -> Result<()> {
        if !node.is_element() {
            return Ok(());
        }

        let tag_name = node.tag_name().name();

        match tag_name {
            "TN" => {
                // Full type definition - extract RefId and register it
                if let Some(ref_id) = node.attribute("RefId") {
                    trace!(ref_id, "Processing TN with RefId");
                    // Process children to get <T> elements
                    self.visit_children(node.children(), context)?;

                    let ps_type = PsType {
                        type_names: self.type_names.clone(),
                    };

                    // Register this type in the context
                    context.register_type(ref_id.to_string(), ps_type.clone());
                    self.resolved_type = Some(ps_type);
                } else {
                    trace!("Processing TN without RefId");
                    // TN without RefId - just process children
                    self.visit_children(node.children(), context)?;
                    self.resolved_type = Some(PsType {
                        type_names: self.type_names.clone(),
                    });
                }
            }
            "TNRef" => {
                // Type reference - look up existing type definition
                if let Some(ref_id) = node.attribute("RefId") {
                    trace!("Processing TNRef with RefId={}", ref_id);
                    if let Some(ps_type) = context.get_type(ref_id) {
                        trace!("Successfully resolved TNRef RefId={}", ref_id);
                        self.resolved_type = Some(ps_type.clone());
                    } else {
                        trace!("Failed to resolve TNRef RefId={}", ref_id);
                        return Err(ironposh_xml::XmlError::GenericError(format!(
                            "Type reference {ref_id} not found"
                        )));
                    }
                } else {
                    trace!("TNRef missing RefId attribute");
                    return Err(ironposh_xml::XmlError::GenericError(
                        "TNRef missing RefId attribute".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ironposh_xml::XmlError::UnexpectedTag(format!(
                    "Unexpected tag in PsType: {tag_name}"
                )));
            }
        }

        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
        _context: &mut DeserializationContext,
    ) -> Result<()> {
        for child in children {
            if child.is_element()
                && child.tag_name().name() == "T"
                && let Some(text) = child.text()
            {
                self.type_names.push(Cow::Owned(text.to_string()));
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value> {
        self.resolved_type
            .ok_or_else(|| ironposh_xml::XmlError::GenericError("No PsType resolved".to_string()))
    }
}

impl<'a> PsXmlDeserialize<'a> for PsType {
    type Visitor = PsTypeContextVisitor<'a>;

    fn visitor_with_context() -> Self::Visitor {
        PsTypeContextVisitor::new()
    }
}

/// Context-aware ComplexObject visitor that uses context for type resolution
#[derive(Default)]
pub struct ComplexObjectContextVisitor<'a> {
    type_def: Option<PsType>,
    to_string: Option<String>,
    content: ComplexObjectContent,
    adapted_properties: BTreeMap<String, PsProperty>,
    extended_properties: BTreeMap<String, PsProperty>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl ComplexObjectContextVisitor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> PsXmlVisitor<'a> for ComplexObjectContextVisitor<'a> {
    type Value = ComplexObject;

    fn visit_node(
        &mut self,
        node: ironposh_xml::parser::Node<'a, 'a>,
        context: &mut DeserializationContext,
    ) -> Result<()> {
        if !node.is_element() {
            return Ok(());
        }

        let tag_name = node.tag_name().name();

        if tag_name == "Obj" {
            let ref_id = node.attribute("RefId");
            trace!("Processing Obj with RefId={:?}", ref_id);
            // Process children of the Obj element
            self.visit_children(node.children(), context)?;

            // If this object has a RefId, register it in the context
            if let Some(ref_id) = ref_id {
                let obj = ComplexObject {
                    type_def: self.type_def.clone(),
                    to_string: self.to_string.clone(),
                    content: self.content.clone(),
                    adapted_properties: self.adapted_properties.clone(),
                    extended_properties: self.extended_properties.clone(),
                };
                context.register_object(ref_id.to_string(), obj);
            }
        }

        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
        context: &mut DeserializationContext,
    ) -> Result<()> {
        for child in children {
            if !child.is_element() {
                continue;
            }

            let tag_name = child.tag_name().name();

            match tag_name {
                "TN" | "TNRef" => {
                    // Use context-aware type deserialization
                    let ps_type = PsType::from_node_with_context(child, context)?;
                    self.type_def = Some(ps_type);
                }
                "ToString" => {
                    if let Some(text) = child.text() {
                        self.to_string = Some(text.to_string());
                    }
                }
                // Handle primitive content for ExtendedPrimitive objects
                "S" | "B" | "I32" | "U32" | "I64" | "U64" | "G" | "C" | "Nil" | "BA"
                | "Version" | "DT" => {
                    let primitive = PsPrimitiveValue::from_node(child)?;
                    self.content = ComplexObjectContent::ExtendedPrimitive(primitive);
                }
                // Handle containers with context
                "STK" | "QUE" | "LST" | "DCT" => {
                    let container = Container::from_node_with_context(child, context)?;
                    self.content = ComplexObjectContent::Container(container);
                }
                "Props" => {
                    // Parse adapted properties with context
                    for prop_child in child.children() {
                        if prop_child.is_element() {
                            let prop = PsProperty::from_node_with_context(prop_child, context)?;
                            self.adapted_properties.insert(prop.name.clone(), prop);
                        }
                    }
                }
                "MS" => {
                    // Parse extended properties with context
                    for prop_child in child.children() {
                        if prop_child.is_element() {
                            let prop = PsProperty::from_node_with_context(prop_child, context)?;
                            self.extended_properties.insert(prop.name.clone(), prop);
                        }
                    }
                }
                _ => {
                    // Unknown element - could be part of content or should be ignored
                    // For now, we'll ignore unknown elements
                }
            }
        }

        // Post-process to detect enum content
        if let Some(type_def) = &self.type_def
            && type_def.type_names.iter().any(|name| name.contains("Enum"))
            && let ComplexObjectContent::ExtendedPrimitive(PsPrimitiveValue::I32(value)) =
                &self.content
        {
            self.content = ComplexObjectContent::PsEnums(PsEnums { value: *value });
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value> {
        Ok(ComplexObject {
            type_def: self.type_def,
            to_string: self.to_string,
            content: self.content,
            adapted_properties: self.adapted_properties,
            extended_properties: self.extended_properties,
        })
    }
}

impl<'a> PsXmlDeserialize<'a> for ComplexObject {
    type Visitor = ComplexObjectContextVisitor<'a>;

    fn visitor_with_context() -> Self::Visitor {
        ComplexObjectContextVisitor::new()
    }
}

/// Context-aware PsValue visitor
#[derive(Default)]
pub struct PsValueContextVisitor<'a> {
    value: Option<PsValue>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl PsValueContextVisitor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> PsXmlVisitor<'a> for PsValueContextVisitor<'a> {
    type Value = PsValue;

    fn visit_node(
        &mut self,
        node: ironposh_xml::parser::Node<'a, 'a>,
        context: &mut DeserializationContext,
    ) -> Result<()> {
        if !node.is_element() {
            return Ok(());
        }

        let tag_name = node.tag_name().name();

        match tag_name {
            // Handle primitive values
            "S" | "B" | "I32" | "U32" | "I64" | "U64" | "G" | "C" | "Nil" | "BA" | "Version"
            | "DT" => {
                let primitive = PsPrimitiveValue::from_node(node)?;
                self.value = Some(PsValue::Primitive(primitive));
            }
            // Handle complex objects with context
            "Obj" => {
                let complex_obj = ComplexObject::from_node_with_context(node, context)?;
                self.value = Some(PsValue::Object(complex_obj));
            }
            // Handle object references
            "Ref" => {
                if let Some(ref_id) = node.attribute("RefId") {
                    trace!("Processing Ref with RefId={}", ref_id);
                    if let Some(complex_obj) = context.get_object(ref_id) {
                        trace!("Successfully resolved object reference RefId={}", ref_id);
                        self.value = Some(PsValue::Object(complex_obj.clone()));
                    } else {
                        trace!("Failed to resolve object reference RefId={}", ref_id);
                        return Err(ironposh_xml::XmlError::GenericError(format!(
                            "Object reference {ref_id} not found"
                        )));
                    }
                } else {
                    trace!("Ref missing RefId attribute");
                    return Err(ironposh_xml::XmlError::GenericError(
                        "Ref missing RefId attribute".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ironposh_xml::XmlError::UnexpectedTag(format!(
                    "Unexpected tag for PsValue: {tag_name}"
                )));
            }
        }

        Ok(())
    }

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
        _context: &mut DeserializationContext,
    ) -> Result<()> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Value> {
        self.value
            .ok_or_else(|| ironposh_xml::XmlError::GenericError("No PsValue found".to_string()))
    }
}

impl<'a> PsXmlDeserialize<'a> for PsValue {
    type Visitor = PsValueContextVisitor<'a>;

    fn visitor_with_context() -> Self::Visitor {
        PsValueContextVisitor::new()
    }
}

/// Context-aware Container visitor
#[derive(Default)]
pub struct ContainerContextVisitor<'a> {
    container: Option<Container>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl ContainerContextVisitor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> PsXmlVisitor<'a> for ContainerContextVisitor<'a> {
    type Value = Container;

    fn visit_node(
        &mut self,
        node: ironposh_xml::parser::Node<'a, 'a>,
        context: &mut DeserializationContext,
    ) -> Result<()> {
        if !node.is_element() {
            return Ok(());
        }

        let tag_name = node.tag_name().name();

        match tag_name {
            "STK" => {
                let mut values = Vec::new();
                for child in node.children() {
                    if child.is_element() {
                        let value = PsValue::from_node_with_context(child, context)?;
                        values.push(value);
                    }
                }
                self.container = Some(Container::Stack(values));
            }
            "QUE" => {
                let mut values = Vec::new();
                for child in node.children() {
                    if child.is_element() {
                        let value = PsValue::from_node_with_context(child, context)?;
                        values.push(value);
                    }
                }
                self.container = Some(Container::Queue(values));
            }
            "LST" => {
                let mut values = Vec::new();
                for child in node.children() {
                    if child.is_element() {
                        let value = PsValue::from_node_with_context(child, context)?;
                        values.push(value);
                    }
                }
                self.container = Some(Container::List(values));
            }
            "DCT" => {
                let mut map = BTreeMap::new();
                for en_child in node.children() {
                    if en_child.is_element() && en_child.tag_name().name() == "En" {
                        let mut key: Option<PsValue> = None;
                        let mut value: Option<PsValue> = None;

                        for entry_child in en_child.children() {
                            if entry_child.is_element()
                                && let Some(n_attr) = entry_child.attribute("N")
                            {
                                match n_attr {
                                    "Key" => {
                                        key = Some(PsValue::from_node_with_context(
                                            entry_child,
                                            context,
                                        )?);
                                    }
                                    "Value" => {
                                        value = Some(PsValue::from_node_with_context(
                                            entry_child,
                                            context,
                                        )?);
                                    }
                                    _ => {}
                                }
                            }
                        }

                        if let (Some(k), Some(v)) = (key, value) {
                            map.insert(k, v);
                        }
                    }
                }
                self.container = Some(Container::Dictionary(map));
            }
            _ => {
                return Err(ironposh_xml::XmlError::UnexpectedTag(format!(
                    "Unexpected container tag: {tag_name}"
                )));
            }
        }

        Ok(())
    }

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
        _context: &mut DeserializationContext,
    ) -> Result<()> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Value> {
        self.container
            .ok_or_else(|| ironposh_xml::XmlError::GenericError("No Container found".to_string()))
    }
}

impl<'a> PsXmlDeserialize<'a> for Container {
    type Visitor = ContainerContextVisitor<'a>;

    fn visitor_with_context() -> Self::Visitor {
        ContainerContextVisitor::new()
    }
}

/// Context-aware PsProperty visitor
#[derive(Default)]
pub struct PsPropertyContextVisitor<'a> {
    name: Option<String>,
    value: Option<PsValue>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl PsPropertyContextVisitor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> PsXmlVisitor<'a> for PsPropertyContextVisitor<'a> {
    type Value = PsProperty;

    fn visit_node(
        &mut self,
        node: ironposh_xml::parser::Node<'a, 'a>,
        context: &mut DeserializationContext,
    ) -> Result<()> {
        if !node.is_element() {
            return Ok(());
        }

        // Extract the N attribute for property name
        if let Some(name_attr) = node.attribute("N") {
            self.name = Some(name_attr.to_string());
        }

        // Parse the value from the node using context
        let value = PsValue::from_node_with_context(node, context)?;
        self.value = Some(value);

        Ok(())
    }

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
        _context: &mut DeserializationContext,
    ) -> Result<()> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Value> {
        let value = self.value.ok_or_else(|| {
            ironposh_xml::XmlError::GenericError("No value found for PsProperty".to_string())
        })?;

        let name = self.name.unwrap_or_default();

        Ok(PsProperty { name, value })
    }
}

impl<'a> PsXmlDeserialize<'a> for PsProperty {
    type Visitor = PsPropertyContextVisitor<'a>;

    fn visitor_with_context() -> Self::Visitor {
        PsPropertyContextVisitor::new()
    }
}
