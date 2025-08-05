use super::{PsObject, PsProperty, PsValue};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use std::collections::HashMap;
use xml::parser::{XmlDeserialize, XmlVisitor};

/// ================================================================================================
/// 1. PsValue Visitor and XmlDeserialize Implementation
/// ================================================================================================

pub struct PsValueVisitor<'a> {
    value: Option<PsValue>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Default for PsValueVisitor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> PsValueVisitor<'a> {
    pub fn new() -> Self {
        Self {
            value: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> XmlVisitor<'a> for PsValueVisitor<'a> {
    type Value = PsValue;

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        if !node.is_element() {
            return Ok(());
        }

        let tag_name = node.tag_name().name();

        match tag_name {
            "S" => {
                let text = node.text().unwrap_or("").to_string();
                self.value = Some(PsValue::Str(text));
            }
            "B" => {
                let text = node.text().unwrap_or("false");
                let bool_val = text.parse::<bool>().map_err(|_| {
                    xml::XmlError::GenericError(format!("Invalid boolean value: {text}"))
                })?;
                self.value = Some(PsValue::Bool(bool_val));
            }
            "I32" => {
                let text = node.text().unwrap_or("0");
                let int_val = text.parse::<i32>().map_err(|_| {
                    xml::XmlError::GenericError(format!("Invalid i32 value: {text}"))
                })?;
                self.value = Some(PsValue::I32(int_val));
            }
            "U32" => {
                let text = node.text().unwrap_or("0");
                let uint_val = text.parse::<u32>().map_err(|_| {
                    xml::XmlError::GenericError(format!("Invalid u32 value: {text}"))
                })?;
                self.value = Some(PsValue::U32(uint_val));
            }
            "I64" => {
                let text = node.text().unwrap_or("0");
                let long_val = text.parse::<i64>().map_err(|_| {
                    xml::XmlError::GenericError(format!("Invalid i64 value: {text}"))
                })?;
                self.value = Some(PsValue::I64(long_val));
            }
            "G" => {
                let text = node.text().unwrap_or("").to_string();
                self.value = Some(PsValue::Guid(text));
            }
            "Nil" => {
                self.value = Some(PsValue::Nil);
            }
            "BA" => {
                let text = node.text().unwrap_or("");
                let bytes = B64.decode(text).map_err(|_| {
                    xml::XmlError::GenericError(format!("Invalid base64 data: {text}"))
                })?;
                self.value = Some(PsValue::Bytes(bytes));
            }
            "Version" => {
                let text = node.text().unwrap_or("").to_string();
                self.value = Some(PsValue::Version(text));
            }
            "Obj" => {
                let obj = PsObject::from_node(node)?;
                self.value = Some(PsValue::Object(obj));
            }
            _ => {
                return Err(xml::XmlError::UnexpectedTag(tag_name.to_string()));
            }
        }

        Ok(())
    }

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        // PsValue typically doesn't need to process children separately
        // since visit_node handles the element content
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        self.value
            .ok_or_else(|| xml::XmlError::GenericError("No PsValue found".to_string()))
    }
}

impl<'a> XmlDeserialize<'a> for PsValue {
    type Visitor = PsValueVisitor<'a>;

    fn visitor() -> Self::Visitor {
        PsValueVisitor::new()
    }
}

/// ================================================================================================
/// 2. PsProperty Visitor and XmlDeserialize Implementation
/// ================================================================================================

pub struct PsPropertyVisitor<'a> {
    name: Option<String>,
    ref_id: Option<u32>,
    value: Option<PsValue>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Default for PsPropertyVisitor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> PsPropertyVisitor<'a> {
    pub fn new() -> Self {
        Self {
            name: None,
            ref_id: None,
            value: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> XmlVisitor<'a> for PsPropertyVisitor<'a> {
    type Value = PsProperty;

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        if !node.is_element() {
            return Ok(());
        }

        // Extract attributes from the node
        if let Some(name_attr) = node.attribute("N") {
            self.name = Some(name_attr.to_string());
        }

        if let Some(ref_id_attr) = node.attribute("RefId") {
            let ref_id = ref_id_attr.parse::<u32>().map_err(|_| {
                xml::XmlError::GenericError(format!("Invalid RefId value: {ref_id_attr}"))
            })?;
            self.ref_id = Some(ref_id);
        }

        // Parse the value from the node itself
        let value = PsValue::from_node(node)?;
        self.value = Some(value);

        Ok(())
    }

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        // PsProperty handles its content through visit_node
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        let value = self.value.ok_or_else(|| {
            xml::XmlError::GenericError("No value found for PsProperty".to_string())
        })?;

        Ok(PsProperty {
            name: self.name,
            ref_id: self.ref_id,
            value,
        })
    }
}

impl<'a> XmlDeserialize<'a> for PsProperty {
    type Visitor = PsPropertyVisitor<'a>;

    fn visitor() -> Self::Visitor {
        PsPropertyVisitor::new()
    }
}

/// ================================================================================================
/// 3. PsObject Visitor and XmlDeserialize Implementation
/// ================================================================================================

pub struct PsObjectVisitor<'a> {
    ref_id: Option<u32>,
    type_names: Option<Vec<String>>,
    tn_ref: Option<u32>,
    props: Vec<PsProperty>,
    ms: Vec<PsProperty>,
    lst: Vec<PsProperty>,
    dct: HashMap<PsValue, PsValue>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Default for PsObjectVisitor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> PsObjectVisitor<'a> {
    pub fn new() -> Self {
        Self {
            ref_id: None,
            type_names: None,
            tn_ref: None,
            props: Vec::new(),
            ms: Vec::new(),
            lst: Vec::new(),
            dct: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> XmlVisitor<'a> for PsObjectVisitor<'a> {
    type Value = PsObject;

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        if !node.is_element() {
            return Ok(());
        }

        let tag_name = node.tag_name().name();

        if tag_name == "Obj" {
            // Extract RefId attribute from the Obj element
            if let Some(ref_id_attr) = node.attribute("RefId") {
                let ref_id = ref_id_attr.parse::<u32>().map_err(|_| {
                    xml::XmlError::GenericError(format!("Invalid RefId value: {ref_id_attr}"))
                })?;
                self.ref_id = Some(ref_id);
            }

            // Process children
            self.visit_children(node.children())?;
        }

        Ok(())
    }

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for child in children {
            if !child.is_element() {
                continue;
            }

            let tag_name = child.tag_name().name();

            match tag_name {
                "TN" => {
                    // Parse type names: <TN RefId="0"><T>Type1</T><T>Type2</T></TN>
                    let mut type_names = Vec::new();
                    for t_child in child.children() {
                        if t_child.is_element()
                            && t_child.tag_name().name() == "T"
                            && let Some(text) = t_child.text()
                        {
                            type_names.push(text.to_string());
                        }
                    }
                    self.type_names = Some(type_names);
                }
                "TNRef" => {
                    // Parse TNRef: <TNRef RefId="..."/>
                    if let Some(ref_id_attr) = child.attribute("RefId") {
                        let tn_ref = ref_id_attr.parse::<u32>().map_err(|_| {
                            xml::XmlError::GenericError(format!(
                                "Invalid TNRef RefId value: {ref_id_attr}"
                            ))
                        })?;
                        self.tn_ref = Some(tn_ref);
                    }
                }
                "Props" => {
                    // Parse properties
                    for prop_child in child.children() {
                        if prop_child.is_element() {
                            let prop = PsProperty::from_node(prop_child)?;
                            self.props.push(prop);
                        }
                    }
                }
                "MS" => {
                    // Parse member set
                    for ms_child in child.children() {
                        if ms_child.is_element() {
                            let prop = PsProperty::from_node(ms_child)?;
                            self.ms.push(prop);
                        }
                    }
                }
                "LST" => {
                    // Parse list
                    for lst_child in child.children() {
                        if lst_child.is_element() {
                            let prop = PsProperty::from_node(lst_child)?;
                            self.lst.push(prop);
                        }
                    }
                }
                "DCT" => {
                    // Parse dictionary: <DCT><En><Key>...</Key><Value>...</Value></En>...</DCT>
                    for en_child in child.children() {
                        if en_child.is_element() && en_child.tag_name().name() == "En" {
                            let mut key: Option<PsValue> = None;
                            let mut value: Option<PsValue> = None;

                            for entry_child in en_child.children() {
                                if entry_child.is_element()
                                    && let Some(n_attr) = entry_child.attribute("N")
                                {
                                    match n_attr {
                                        "Key" => {
                                            key = Some(PsValue::from_node(entry_child)?);
                                        }
                                        "Value" => {
                                            value = Some(PsValue::from_node(entry_child)?);
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            if let (Some(k), Some(v)) = (key, value) {
                                self.dct.insert(k, v);
                            }
                        }
                    }
                }
                _ => {
                    // Unknown child element, might be handled differently
                }
            }
        }

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(PsObject {
            ref_id: self.ref_id,
            type_names: self.type_names,
            tn_ref: self.tn_ref,
            props: self.props,
            ms: self.ms,
            lst: self.lst,
            dct: self.dct,
        })
    }
}

impl<'a> XmlDeserialize<'a> for PsObject {
    type Visitor = PsObjectVisitor<'a>;

    fn visitor() -> Self::Visitor {
        PsObjectVisitor::new()
    }
}
