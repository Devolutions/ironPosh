use super::*;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::BufRead;

pub struct PsObjectParser;

impl PsObjectParser {
    pub fn parse_from_xml<R: BufRead>(xml: &str) -> Result<PsObject, Box<dyn std::error::Error>> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) if e.name().as_ref() == b"Obj" => {
                    return Self::parse_object(&mut reader, e);
                }
                Event::Eof => break,
                _ => (),
            }
            buf.clear();
        }
        
        Err("No Obj element found".into())
    }
    
    fn parse_object<R: BufRead>(
        reader: &mut Reader<R>, 
        start_tag: &quick_xml::events::BytesStart
    ) -> Result<PsObject, Box<dyn std::error::Error>> {
        let mut obj = PsObject::new();
        
        // Parse attributes
        for attr in start_tag.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"RefId" => {
                    obj.ref_id = Some(String::from_utf8(attr.value.to_vec())?);
                }
                _ => {}
            }
        }
        
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    match e.name().as_ref() {
                        b"TN" => {
                            obj.type_names = Some(Self::parse_type_names(reader, e)?);
                        }
                        b"TNRef" => {
                            obj.tn_ref = Some(Self::parse_type_name_ref(e)?);
                        }
                        b"ToString" => {
                            obj.to_string = Some(Self::parse_text_content(reader)?);
                        }
                        b"MS" => {
                            obj.ms = Self::parse_property_list(reader)?;
                        }
                        b"Props" => {
                            obj.props = Self::parse_property_list(reader)?;
                        }
                        b"LST" => {
                            obj.lst = Self::parse_property_list(reader)?;
                        }
                        b"DCT" => {
                            obj.dct = Self::parse_dictionary(reader)?;
                        }
                        _ => {
                            // Handle primitive values directly in object (like <I32>0</I32>)
                            let prop = Self::parse_property_value(reader, e)?;
                            obj.ms.push(prop);
                        }
                    }
                }
                Event::Empty(ref e) => {
                    match e.name().as_ref() {
                        b"TNRef" => {
                            obj.tn_ref = Some(Self::parse_type_name_ref(e)?);
                        }
                        b"Nil" => {
                            let prop = Self::parse_empty_property(e)?;
                            obj.ms.push(prop);
                        }
                        _ => {}
                    }
                }
                Event::End(ref e) if e.name().as_ref() == b"Obj" => {
                    break;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        Ok(obj)
    }
    
    fn parse_type_names<R: BufRead>(
        reader: &mut Reader<R>,
        start_tag: &quick_xml::events::BytesStart
    ) -> Result<PsTypeNames, Box<dyn std::error::Error>> {
        let mut type_names = PsTypeNames {
            ref_id: None,
            names: Vec::new(),
        };
        
        // Parse attributes
        for attr in start_tag.attributes() {
            let attr = attr?;
            if attr.key.as_ref() == b"RefId" {
                type_names.ref_id = Some(String::from_utf8(attr.value.to_vec())?);
            }
        }
        
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) if e.name().as_ref() == b"T" => {
                    let type_name = Self::parse_text_content(reader)?;
                    type_names.names.push(type_name);
                }
                Event::End(ref e) if e.name().as_ref() == b"TN" => {
                    break;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        Ok(type_names)
    }
    
    fn parse_type_name_ref(
        start_tag: &quick_xml::events::BytesStart
    ) -> Result<PsTypeNameRef, Box<dyn std::error::Error>> {
        for attr in start_tag.attributes() {
            let attr = attr?;
            if attr.key.as_ref() == b"RefId" {
                return Ok(PsTypeNameRef {
                    ref_id: String::from_utf8(attr.value.to_vec())?,
                });
            }
        }
        Err("TNRef missing RefId attribute".into())
    }
    
    fn parse_property_list<R: BufRead>(
        reader: &mut Reader<R>
    ) -> Result<Vec<PsProperty>, Box<dyn std::error::Error>> {
        let mut properties = Vec::new();
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    let prop = Self::parse_property_value(reader, e)?;
                    properties.push(prop);
                }
                Event::Empty(ref e) => {
                    let prop = Self::parse_empty_property(e)?;
                    properties.push(prop);
                }
                Event::End(ref e) if matches!(e.name().as_ref(), b"MS" | b"Props" | b"LST") => {
                    break;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        Ok(properties)
    }
    
    fn parse_dictionary<R: BufRead>(
        reader: &mut Reader<R>
    ) -> Result<Vec<PsDictionaryEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        let mut buf = Vec::new();
        
        // First, we expect an <En> container
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) if e.name().as_ref() == b"En" => {
                    break;
                }
                Event::End(ref e) if e.name().as_ref() == b"DCT" => {
                    return Ok(entries);
                }
                Event::Eof => return Ok(entries),
                _ => {}
            }
            buf.clear();
        }
        
        // Now parse entries within <En>
        let mut current_key: Option<PsValue> = None;
        let mut current_value: Option<PsValue> = None;
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    let prop = Self::parse_property_value(reader, e)?;
                    if prop.name.as_deref() == Some("Key") {
                        current_key = Some(prop.value);
                    } else if prop.name.as_deref() == Some("Value") {
                        current_value = Some(prop.value);
                    }
                    
                    // If we have both key and value, create an entry
                    if let (Some(key), Some(value)) = (current_key.take(), current_value.take()) {
                        entries.push(PsDictionaryEntry { key, value });
                    }
                }
                Event::Empty(ref e) => {
                    let prop = Self::parse_empty_property(e)?;
                    if prop.name.as_deref() == Some("Key") {
                        current_key = Some(prop.value);
                    } else if prop.name.as_deref() == Some("Value") {
                        current_value = Some(prop.value);
                    }
                    
                    // If we have both key and value, create an entry
                    if let (Some(key), Some(value)) = (current_key.take(), current_value.take()) {
                        entries.push(PsDictionaryEntry { key, value });
                    }
                }
                Event::End(ref e) if e.name().as_ref() == b"En" => {
                    break;
                }
                Event::End(ref e) if e.name().as_ref() == b"DCT" => {
                    break;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        Ok(entries)
    }
    
    fn parse_property_value<R: BufRead>(
        reader: &mut Reader<R>,
        start_tag: &quick_xml::events::BytesStart
    ) -> Result<PsProperty, Box<dyn std::error::Error>> {
        let mut name = None;
        let mut ref_id = None;
        
        // Parse attributes
        for attr in start_tag.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"N" => {
                    name = Some(String::from_utf8(attr.value.to_vec())?);
                }
                b"RefId" => {
                    ref_id = Some(String::from_utf8(attr.value.to_vec())?);
                }
                _ => {}
            }
        }
        
        let tag_name = start_tag.name();
        let value = match tag_name.as_ref() {
            b"S" => PsValue::Str(Self::parse_text_content(reader)?),
            b"C" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::Char(text.chars().next().unwrap_or('\0'))
            }
            b"B" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::Bool(text == "true")
            }
            b"I32" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::Int32(text.parse().unwrap_or(0))
            }
            b"U32" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::UInt32(text.parse().unwrap_or(0))
            }
            b"I64" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::Int64(text.parse().unwrap_or(0))
            }
            b"U64" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::UInt64(text.parse().unwrap_or(0))
            }
            b"Db" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::Double(text.parse().unwrap_or(0.0))
            }
            b"Sg" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::Float(text.parse().unwrap_or(0.0))
            }
            b"G" => {
                let text = Self::parse_text_content(reader)?;
                PsValue::Guid(text.parse().unwrap_or(Uuid::nil()))
            }
            b"BA" => {
                let text = Self::parse_text_content(reader)?;
                let bytes = BASE64.decode(text.as_bytes()).unwrap_or_default();
                PsValue::ByteArray(bytes)
            }
            b"Version" => {
                let text = Self::parse_text_content(reader)?;
                let version = text.parse().unwrap_or_else(|_| Version::new(0, 0, 0));
                PsValue::Version(version)
            }
            b"Obj" => {
                let obj = Self::parse_object(reader, start_tag)?;
                PsValue::Object(obj)
            }
            _ => {
                // Default to string for unknown types
                PsValue::Str(Self::parse_text_content(reader)?)
            }
        };
        
        Ok(PsProperty {
            name,
            ref_id,
            value,
        })
    }
    
    fn parse_empty_property(
        start_tag: &quick_xml::events::BytesStart
    ) -> Result<PsProperty, Box<dyn std::error::Error>> {
        let mut name = None;
        let mut ref_id = None;
        
        // Parse attributes
        for attr in start_tag.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"N" => {
                    name = Some(String::from_utf8(attr.value.to_vec())?);
                }
                b"RefId" => {
                    ref_id = Some(String::from_utf8(attr.value.to_vec())?);
                }
                _ => {}
            }
        }
        
        Ok(PsProperty {
            name,
            ref_id,
            value: PsValue::Nil,
        })
    }
    
    fn parse_text_content<R: BufRead>(
        reader: &mut Reader<R>
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let mut text = String::new();
        
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Text(e) => {
                    let unescaped = e.unescape().map_err(|e| format!("Unescape error: {}", e))?;
                    text.push_str(&unescaped);
                }
                Event::End(_) => {
                    break;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        
        Ok(text)
    }
}
