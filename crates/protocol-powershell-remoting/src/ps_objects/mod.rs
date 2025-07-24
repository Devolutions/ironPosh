use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use quick_xml::events::{Event, BytesStart, BytesEnd, BytesText};
use quick_xml::Writer;
use std::io::Write;
use uuid::Uuid;
use semver::Version;
use chrono::{DateTime, Utc, Duration};

pub mod parser;

/// One PS "primitive" or nested object.
#[derive(Debug, Clone, PartialEq)]
pub enum PsValue {
    Str(String),                    // <S>
    Char(char),                     // <C>
    Bool(bool),                     // <B>
    DateTime(DateTime<Utc>),        // <DT>
    TimeSpan(Duration),             // <TS>
    UByte(u8),                      // <By>
    Byte(i8),                       // <SB>
    UInt16(u16),                    // <U16>
    Int16(i16),                     // <I16>
    UInt32(u32),                    // <U32>
    Int32(i32),                     // <I32>
    UInt64(u64),                    // <U64>
    Int64(i64),                     // <I64>
    Float(f32),                     // <Sg>
    Double(f64),                    // <Db>
    Decimal(String),                // <D> - stored as string to preserve precision
    ByteArray(Vec<u8>),             // <BA>
    Guid(Uuid),                     // <G>
    Uri(String),                    // <URI>
    Nil,                            // <Nil/>
    Version(Version),               // <Version>
    Xml(String),                    // <XD>
    ScriptBlock(String),            // <SBK>
    SecureString(String),           // <SS>
    Object(PsObject),               // <Obj>
}

/// A property wrapper that carries the `N=` and `RefId=` attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct PsProperty {
    pub name: Option<String>,       // N="..."
    pub ref_id: Option<String>,     // RefId="..."
    pub value: PsValue,             // actual payload
}

/// Type names collection for PS objects
#[derive(Debug, Clone, PartialEq)]
pub struct PsTypeNames {
    pub ref_id: Option<String>,
    pub names: Vec<String>,
}

/// Type name reference
#[derive(Debug, Clone, PartialEq)]
pub struct PsTypeNameRef {
    pub ref_id: String,
}

/// Dictionary entry for PS dictionaries
#[derive(Debug, Clone, PartialEq)]
pub struct PsDictionaryEntry {
    pub key: PsValue,
    pub value: PsValue,
}

/// A full <Obj>.
#[derive(Debug, Clone, PartialEq)]
pub struct PsObject {
    pub ref_id: Option<String>,                         // RefId="..."
    pub type_names: Option<PsTypeNames>,                // <TN>
    pub tn_ref: Option<PsTypeNameRef>,                  // <TNRef>
    pub to_string: Option<String>,                      // <ToString>
    pub props: Vec<PsProperty>,                         // <Props>
    pub ms: Vec<PsProperty>,                            // <MS> - member set
    pub lst: Vec<PsProperty>,                           // <LST> - list/array
    pub dct: Vec<PsDictionaryEntry>,                    // <DCT> - dictionary
    
    // Property cache for efficient lookups
    property_cache: Option<HashMap<String, PsProperty>>,
    property_cache_case_insensitive: Option<HashMap<String, PsProperty>>,
}

impl Default for PsObject {
    fn default() -> Self {
        Self::new()
    }
}

impl PsObject {
    pub fn new() -> Self {
        Self {
            ref_id: None,
            type_names: None,
            tn_ref: None,
            to_string: None,
            props: Vec::new(),
            ms: Vec::new(),
            lst: Vec::new(),
            dct: Vec::new(),
            property_cache: None,
            property_cache_case_insensitive: None,
        }
    }

    pub fn with_ref_id(ref_id: String) -> Self {
        Self {
            ref_id: Some(ref_id),
            ..Self::new()
        }
    }

    /// Get a property by name (case sensitive or insensitive)
    pub fn get_property(&mut self, name: &str, case_sensitive: bool) -> Option<&PsValue> {
        self.get_ps_property(name, case_sensitive).map(|p| &p.value)
    }

    /// Get a PS property by name
    pub fn get_ps_property(&mut self, name: &str, case_sensitive: bool) -> Option<&PsProperty> {
        if case_sensitive {
            if self.property_cache.is_none() {
                self.build_property_cache();
            }
            self.property_cache.as_ref()?.get(name)
        } else {
            if self.property_cache_case_insensitive.is_none() {
                self.build_property_cache_case_insensitive();
            }
            self.property_cache_case_insensitive.as_ref()?.get(&name.to_lowercase())
        }
    }

    /// Try to get an array of values of type T
    pub fn try_get_array<T>(&mut self, name: &str) -> Option<Vec<T>>
    where
        T: TryFrom<PsValue>,
    {
        if let Some(PsValue::Object(obj)) = self.get_property(name, false) {
            let obj = obj.clone();
            let mut result = Vec::new();
            for prop in &obj.lst {
                if let Ok(value) = T::try_from(prop.value.clone()) {
                    result.push(value);
                }
            }
            if !result.is_empty() {
                return Some(result);
            }
        }
        None
    }

    /// Try to get an enum value
    pub fn try_get_enum<T>(&mut self, name: &str) -> Option<T>
    where
        T: TryFrom<i32>,
    {
        if let Some(PsValue::Object(obj)) = self.get_property(name, false) {
            let mut obj = obj.clone();
            if let Some(PsValue::Int32(value)) = obj.get_property("", false) {
                return T::try_from(*value).ok();
            }
        }
        None
    }

    fn build_property_cache(&mut self) {
        let mut cache = HashMap::with_capacity(self.ms.len() + self.props.len());
        
        for prop in &self.ms {
            if let Some(name) = &prop.name {
                cache.insert(name.clone(), prop.clone());
            }
        }
        
        for prop in &self.props {
            if let Some(name) = &prop.name {
                cache.insert(name.clone(), prop.clone());
            }
        }
        
        self.property_cache = Some(cache);
    }

    fn build_property_cache_case_insensitive(&mut self) {
        let mut cache = HashMap::with_capacity(self.ms.len() + self.props.len());
        
        for prop in &self.ms {
            if let Some(name) = &prop.name {
                cache.insert(name.to_lowercase(), prop.clone());
            }
        }
        
        for prop in &self.props {
            if let Some(name) = &prop.name {
                cache.insert(name.to_lowercase(), prop.clone());
            }
        }
        
        self.property_cache_case_insensitive = Some(cache);
    }
}

impl PsProperty {
    pub fn new(name: Option<String>, value: PsValue) -> Self {
        Self {
            name,
            ref_id: None,
            value,
        }
    }

    pub fn with_ref_id(name: Option<String>, ref_id: String, value: PsValue) -> Self {
        Self {
            name,
            ref_id: Some(ref_id),
            value,
        }
    }
}

impl PsValue {
    /// Get the XML element name for this value type
    pub fn element_name(&self) -> &'static str {
        match self {
            PsValue::Str(_) => "S",
            PsValue::Char(_) => "C",
            PsValue::Bool(_) => "B",
            PsValue::DateTime(_) => "DT",
            PsValue::TimeSpan(_) => "TS",
            PsValue::UByte(_) => "By",
            PsValue::Byte(_) => "SB",
            PsValue::UInt16(_) => "U16",
            PsValue::Int16(_) => "I16",
            PsValue::UInt32(_) => "U32",
            PsValue::Int32(_) => "I32",
            PsValue::UInt64(_) => "U64",
            PsValue::Int64(_) => "I64",
            PsValue::Float(_) => "Sg",
            PsValue::Double(_) => "Db",
            PsValue::Decimal(_) => "D",
            PsValue::ByteArray(_) => "BA",
            PsValue::Guid(_) => "G",
            PsValue::Uri(_) => "URI",
            PsValue::Nil => "Nil",
            PsValue::Version(_) => "Version",
            PsValue::Xml(_) => "XD",
            PsValue::ScriptBlock(_) => "SBK",
            PsValue::SecureString(_) => "SS",
            PsValue::Object(_) => "Obj",
        }
    }

    /// Write this value as XML
    pub fn write_xml<W: Write>(&self, writer: &mut Writer<W>, property: &PsProperty) -> Result<(), quick_xml::Error> {
        let tag_name = self.element_name();
        
        let mut start_tag = BytesStart::new(tag_name);
        
        // Add name attribute if present
        if let Some(name) = &property.name {
            start_tag.push_attribute(("N", name.as_str()));
        }
        
        // Add RefId attribute if present
        if let Some(ref_id) = &property.ref_id {
            start_tag.push_attribute(("RefId", ref_id.as_str()));
        }

        match self {
            PsValue::Nil => {
                writer.write_event(Event::Empty(start_tag))?;
            }
            PsValue::Object(obj) => {
                obj.write_xml(writer)?;
            }
            _ => {
                writer.write_event(Event::Start(start_tag.clone()))?;
                
                let text_content = match self {
                    PsValue::Str(s) => s.clone(),
                    PsValue::Char(c) => c.to_string(),
                    PsValue::Bool(b) => b.to_string(),
                    PsValue::DateTime(dt) => dt.to_rfc3339(),
                    PsValue::TimeSpan(dur) => format!("PT{}S", dur.num_seconds()),
                    PsValue::UByte(b) => b.to_string(),
                    PsValue::Byte(b) => b.to_string(),
                    PsValue::UInt16(u) => u.to_string(),
                    PsValue::Int16(i) => i.to_string(),
                    PsValue::UInt32(u) => u.to_string(),
                    PsValue::Int32(i) => i.to_string(),
                    PsValue::UInt64(u) => u.to_string(),
                    PsValue::Int64(i) => i.to_string(),
                    PsValue::Float(f) => f.to_string(),
                    PsValue::Double(d) => d.to_string(),
                    PsValue::Decimal(d) => d.clone(),
                    PsValue::ByteArray(bytes) => BASE64.encode(bytes),
                    PsValue::Guid(g) => g.to_string(),
                    PsValue::Uri(u) => u.clone(),
                    PsValue::Version(v) => v.to_string(),
                    PsValue::Xml(x) => x.clone(),
                    PsValue::ScriptBlock(s) => s.clone(),
                    PsValue::SecureString(s) => s.clone(),
                    _ => unreachable!(),
                };
                
                writer.write_event(Event::Text(BytesText::new(&text_content)))?;
                writer.write_event(Event::End(BytesEnd::new(tag_name)))?;
            }
        }
        
        Ok(())
    }
}

impl PsObject {
    /// Write this object as XML
    pub fn write_xml<W: Write>(&self, writer: &mut Writer<W>) -> Result<(), quick_xml::Error> {
        let mut obj_start = BytesStart::new("Obj");
        
        if let Some(ref_id) = &self.ref_id {
            obj_start.push_attribute(("RefId", ref_id.as_str()));
        }
        
        writer.write_event(Event::Start(obj_start.clone()))?;
        
        // Write type names
        if let Some(type_names) = &self.type_names {
            let mut tn_start = BytesStart::new("TN");
            if let Some(ref_id) = &type_names.ref_id {
                tn_start.push_attribute(("RefId", ref_id.as_str()));
            }
            writer.write_event(Event::Start(tn_start.clone()))?;
            
            for name in &type_names.names {
                writer.write_event(Event::Start(BytesStart::new("T")))?;
                writer.write_event(Event::Text(BytesText::new(name)))?;
                writer.write_event(Event::End(BytesEnd::new("T")))?;
            }
            
            writer.write_event(Event::End(BytesEnd::new("TN")))?;
        }
        
        // Write type name reference
        if let Some(tn_ref) = &self.tn_ref {
            let mut tnref_start = BytesStart::new("TNRef");
            tnref_start.push_attribute(("RefId", tn_ref.ref_id.as_str()));
            writer.write_event(Event::Empty(tnref_start))?;
        }
        
        // Write ToString
        if let Some(to_string) = &self.to_string {
            writer.write_event(Event::Start(BytesStart::new("ToString")))?;
            writer.write_event(Event::Text(BytesText::new(to_string)))?;
            writer.write_event(Event::End(BytesEnd::new("ToString")))?;
        }
        
        // Write Props
        if !self.props.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("Props")))?;
            for prop in &self.props {
                prop.value.write_xml(writer, prop)?;
            }
            writer.write_event(Event::End(BytesEnd::new("Props")))?;
        }
        
        // Write MS (Member Set)
        if !self.ms.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("MS")))?;
            for prop in &self.ms {
                prop.value.write_xml(writer, prop)?;
            }
            writer.write_event(Event::End(BytesEnd::new("MS")))?;
        }
        
        // Write LST (List)
        if !self.lst.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("LST")))?;
            for prop in &self.lst {
                prop.value.write_xml(writer, prop)?;
            }
            writer.write_event(Event::End(BytesEnd::new("LST")))?;
        }
        
        // Write DCT (Dictionary)
        if !self.dct.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("DCT")))?;
            writer.write_event(Event::Start(BytesStart::new("En")))?;
            for entry in &self.dct {
                // Key
                let key_prop = PsProperty::new(Some("Key".to_string()), entry.key.clone());
                entry.key.write_xml(writer, &key_prop)?;
                // Value
                let value_prop = PsProperty::new(Some("Value".to_string()), entry.value.clone());
                entry.value.write_xml(writer, &value_prop)?;
            }
            writer.write_event(Event::End(BytesEnd::new("En")))?;
            writer.write_event(Event::End(BytesEnd::new("DCT")))?;
        }
        
        writer.write_event(Event::End(BytesEnd::new("Obj")))?;
        Ok(())
    }
}

// Convenience constructors for common PS types
impl PsValue {
    pub fn string<S: Into<String>>(s: S) -> Self {
        PsValue::Str(s.into())
    }
    
    pub fn int32(i: i32) -> Self {
        PsValue::Int32(i)
    }
    
    pub fn bool(b: bool) -> Self {
        PsValue::Bool(b)
    }
    
    pub fn nil() -> Self {
        PsValue::Nil
    }
    
    pub fn guid(g: Uuid) -> Self {
        PsValue::Guid(g)
    }
    
    pub fn bytes(b: Vec<u8>) -> Self {
        PsValue::ByteArray(b)
    }
}

// Conversion traits for easier usage
impl TryFrom<PsValue> for String {
    type Error = ();
    
    fn try_from(value: PsValue) -> Result<Self, Self::Error> {
        match value {
            PsValue::Str(s) => Ok(s),
            _ => Err(()),
        }
    }
}

impl TryFrom<PsValue> for i32 {
    type Error = ();
    
    fn try_from(value: PsValue) -> Result<Self, Self::Error> {
        match value {
            PsValue::Int32(i) => Ok(i),
            _ => Err(()),
        }
    }
}

impl TryFrom<PsValue> for bool {
    type Error = ();
    
    fn try_from(value: PsValue) -> Result<Self, Self::Error> {
        match value {
            PsValue::Bool(b) => Ok(b),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_ps_object_creation() {
        let mut obj = PsObject::with_ref_id("1".to_string());
        obj.ms.push(PsProperty::new(
            Some("Name".to_string()),
            PsValue::string("TestObject")
        ));
        obj.ms.push(PsProperty::new(
            Some("Count".to_string()),
            PsValue::int32(42)
        ));
        
        assert_eq!(obj.ref_id, Some("1".to_string()));
        assert_eq!(obj.ms.len(), 2);
    }

    #[test]
    fn test_ps_object_xml_serialization() {
        let mut obj = PsObject::with_ref_id("2".to_string());
        obj.type_names = Some(PsTypeNames {
            ref_id: Some("0".to_string()),
            names: vec![
                "System.Management.Automation.Runspaces.PSThreadOptions".to_string(),
                "System.Enum".to_string(),
                "System.ValueType".to_string(),
                "System.Object".to_string(),
            ],
        });
        obj.ms.push(PsProperty::new(
            Some("ToString".to_string()),
            PsValue::string("Default")
        ));
        obj.ms.push(PsProperty::new(
            None,
            PsValue::int32(0)
        ));

        let mut output = Vec::new();
        let mut writer = Writer::new(Cursor::new(&mut output));
        obj.write_xml(&mut writer).unwrap();

        let xml_string = String::from_utf8(output).unwrap();
        assert!(xml_string.contains("Obj RefId=\"2\""));
        assert!(xml_string.contains("<TN RefId=\"0\">"));
        assert!(xml_string.contains("<T>System.Management.Automation.Runspaces.PSThreadOptions</T>"));
        assert!(xml_string.contains("<S N=\"ToString\">Default</S>"));
        assert!(xml_string.contains("<I32>0</I32>"));
    }

    #[test]
    fn test_property_lookup() {
        let mut obj = PsObject::new();
        obj.ms.push(PsProperty::new(
            Some("CaseSensitive".to_string()),
            PsValue::string("test")
        ));
        obj.ms.push(PsProperty::new(
            Some("caseinsensitive".to_string()),
            PsValue::int32(123)
        ));

        // Case sensitive lookup
        assert!(obj.get_property("CaseSensitive", true).is_some());
        assert!(obj.get_property("casesensitive", true).is_none());

        // Case insensitive lookup
        assert!(obj.get_property("CASESENSITIVE", false).is_some());
        assert!(obj.get_property("CASEINSENSITIVE", false).is_some());
    }
}