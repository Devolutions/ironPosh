use ironposh_client_core::host;
use ironposh_psrp::{
    ComplexObject, ComplexObjectContent, Container, PsEnums, PsPrimitiveValue, PsProperty, PsType,
    PsValue,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use tsify::Tsify;

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsCoordinates {
    pub x: i32,
    pub y: i32,
}

impl From<host::Coordinates> for JsCoordinates {
    fn from(value: host::Coordinates) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<JsCoordinates> for host::Coordinates {
    fn from(value: JsCoordinates) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsSize {
    pub width: i32,
    pub height: i32,
}

impl From<host::Size> for JsSize {
    fn from(value: host::Size) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<JsSize> for host::Size {
    fn from(value: JsSize) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsRectangle {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl From<host::Rectangle> for JsRectangle {
    fn from(value: host::Rectangle) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

impl From<JsRectangle> for host::Rectangle {
    fn from(value: JsRectangle) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsBufferCell {
    /// Single-character string.
    pub character: String,
    pub foreground: i32,
    pub background: i32,
    pub flags: i32,
}

impl From<host::BufferCell> for JsBufferCell {
    fn from(value: host::BufferCell) -> Self {
        Self {
            character: value.character.to_string(),
            foreground: value.foreground,
            background: value.background,
            flags: value.flags,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsKeyInfo {
    pub virtual_key_code: i32,
    /// Single-character string.
    pub character: String,
    pub control_key_state: i32,
    pub key_down: bool,
}

impl From<host::KeyInfo> for JsKeyInfo {
    fn from(value: host::KeyInfo) -> Self {
        Self {
            virtual_key_code: value.virtual_key_code,
            character: value.character.to_string(),
            control_key_state: value.control_key_state,
            key_down: value.key_down,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsProgressRecord {
    pub activity: String,
    pub status_description: String,
    pub current_operation: String,
    pub activity_id: i32,
    pub parent_activity_id: i32,
    pub percent_complete: i32,
    pub seconds_remaining: i32,
    pub record_type: i32,
}

impl From<host::ProgressRecord> for JsProgressRecord {
    fn from(value: host::ProgressRecord) -> Self {
        Self {
            activity: value.activity,
            status_description: value.status_description,
            current_operation: value.current_operation,
            activity_id: value.activity_id,
            parent_activity_id: value.parent_activity_id,
            percent_complete: value.percent_complete,
            seconds_remaining: value.seconds_remaining,
            record_type: value.record_type,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsFieldDescription {
    pub name: String,
    pub label: String,
    pub help_message: String,
    pub is_mandatory: bool,
    pub parameter_type: String,
    /// Debug string for the default value (if any). The typed value may be added later.
    pub default_value_debug: Option<String>,
}

impl From<host::FieldDescription> for JsFieldDescription {
    fn from(value: host::FieldDescription) -> Self {
        Self {
            name: value.name,
            label: value.label,
            help_message: value.help_message,
            is_mandatory: value.is_mandatory,
            parameter_type: value.parameter_type,
            default_value_debug: value.default_value.as_ref().map(ToString::to_string),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsChoiceDescription {
    pub label: String,
    pub help_message: String,
}

impl From<host::ChoiceDescription> for JsChoiceDescription {
    fn from(value: host::ChoiceDescription) -> Self {
        Self {
            label: value.label,
            help_message: value.help_message,
        }
    }
}

/// Accept either bytes (for SecureString payload bytes) or a string (to be encoded by the host).
#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(untagged)]
pub enum JsBytesOrString {
    Bytes(Vec<u8>),
    Text(String),
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPSCredential {
    #[serde(alias = "UserName", alias = "userName")]
    pub user_name: String,
    #[serde(alias = "Password", alias = "password")]
    pub password: JsBytesOrString,
}

// =============================================================================
// Optional structured parameter payloads (for backwards compatible enrichment)
// =============================================================================

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsWriteProgressStructured {
    pub source_id: i64,
    pub record: JsProgressRecord,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPromptStructured {
    pub caption: String,
    pub message: String,
    pub fields: Vec<JsFieldDescription>,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPromptForChoiceStructured {
    pub caption: String,
    pub message: String,
    pub choices: Vec<JsChoiceDescription>,
    pub default_choice: i32,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPromptForChoiceMultipleSelectionStructured {
    pub caption: String,
    pub message: String,
    pub choices: Vec<JsChoiceDescription>,
    pub default_choices: Vec<i32>,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsSetBufferContentsStructured {
    pub rect: JsRectangle,
    pub cell: JsBufferCell,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsGetBufferContentsStructured {
    pub rect: JsRectangle,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsScrollBufferContentsStructured {
    pub source: JsRectangle,
    pub destination: JsCoordinates,
    pub clip: JsRectangle,
    pub fill: JsBufferCell,
}

// =============================================================================
// PsValue (structured, TS-friendly)
// =============================================================================

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum JsPsValue {
    Primitive(JsPsPrimitiveValue),
    Object(JsComplexObject),
}

impl From<PsValue> for JsPsValue {
    fn from(value: PsValue) -> Self {
        match value {
            PsValue::Primitive(p) => Self::Primitive(JsPsPrimitiveValue::from(p)),
            PsValue::Object(o) => Self::Object(JsComplexObject::from(o)),
        }
    }
}

impl TryFrom<JsPsValue> for PsValue {
    type Error = String;

    fn try_from(value: JsPsValue) -> Result<Self, Self::Error> {
        match value {
            JsPsValue::Primitive(p) => Ok(Self::Primitive(PsPrimitiveValue::try_from(p)?)),
            JsPsValue::Object(o) => Ok(Self::Object(ComplexObject::try_from(o)?)),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum JsPsPrimitiveValue {
    Str(String),
    Bool(bool),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Guid(String),
    /// Single-character string.
    Char(String),
    Nil,
    Bytes(Vec<u8>),
    SecureString(Vec<u8>),
    Version(String),
    DateTime(String),
    TimeSpan(String),
}

impl From<PsPrimitiveValue> for JsPsPrimitiveValue {
    fn from(value: PsPrimitiveValue) -> Self {
        match value {
            PsPrimitiveValue::Str(s) => Self::Str(s),
            PsPrimitiveValue::Bool(b) => Self::Bool(b),
            PsPrimitiveValue::I32(i) => Self::I32(i),
            PsPrimitiveValue::U32(u) => Self::U32(u),
            PsPrimitiveValue::I64(i) => Self::I64(i),
            PsPrimitiveValue::U64(u) => Self::U64(u),
            PsPrimitiveValue::Guid(g) => Self::Guid(g),
            PsPrimitiveValue::Char(c) => Self::Char(c.to_string()),
            PsPrimitiveValue::Nil => Self::Nil,
            PsPrimitiveValue::Bytes(b) => Self::Bytes(b),
            PsPrimitiveValue::SecureString(b) => Self::SecureString(b),
            PsPrimitiveValue::Version(v) => Self::Version(v),
            PsPrimitiveValue::DateTime(d) => Self::DateTime(d),
            PsPrimitiveValue::TimeSpan(t) => Self::TimeSpan(t),
        }
    }
}

impl TryFrom<JsPsPrimitiveValue> for PsPrimitiveValue {
    type Error = String;

    fn try_from(value: JsPsPrimitiveValue) -> Result<Self, Self::Error> {
        Ok(match value {
            JsPsPrimitiveValue::Str(s) => Self::Str(s),
            JsPsPrimitiveValue::Bool(b) => Self::Bool(b),
            JsPsPrimitiveValue::I32(i) => Self::I32(i),
            JsPsPrimitiveValue::U32(u) => Self::U32(u),
            JsPsPrimitiveValue::I64(i) => Self::I64(i),
            JsPsPrimitiveValue::U64(u) => Self::U64(u),
            JsPsPrimitiveValue::Guid(g) => Self::Guid(g),
            JsPsPrimitiveValue::Char(s) => {
                let ch = s
                    .chars()
                    .next()
                    .ok_or_else(|| "expected single-character string for Char".to_string())?;
                Self::Char(ch)
            }
            JsPsPrimitiveValue::Nil => Self::Nil,
            JsPsPrimitiveValue::Bytes(b) => Self::Bytes(b),
            JsPsPrimitiveValue::SecureString(b) => Self::SecureString(b),
            JsPsPrimitiveValue::Version(v) => Self::Version(v),
            JsPsPrimitiveValue::DateTime(d) => Self::DateTime(d),
            JsPsPrimitiveValue::TimeSpan(t) => Self::TimeSpan(t),
        })
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPsType {
    pub type_names: Vec<String>,
}

impl From<PsType> for JsPsType {
    fn from(value: PsType) -> Self {
        Self {
            type_names: value
                .type_names
                .into_iter()
                .map(|t| t.to_string())
                .collect(),
        }
    }
}

impl From<JsPsType> for PsType {
    fn from(value: JsPsType) -> Self {
        Self {
            type_names: value.type_names.into_iter().map(Cow::Owned).collect(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPsProperty {
    pub name: String,
    pub value: JsPsValue,
}

impl From<PsProperty> for JsPsProperty {
    fn from(value: PsProperty) -> Self {
        Self {
            name: value.name,
            value: JsPsValue::from(value.value),
        }
    }
}

impl TryFrom<JsPsProperty> for PsProperty {
    type Error = String;

    fn try_from(value: JsPsProperty) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            value: PsValue::try_from(value.value)?,
        })
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsDictionaryEntry {
    pub key: JsPsValue,
    pub value: JsPsValue,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum JsContainer {
    Stack(Vec<JsPsValue>),
    Queue(Vec<JsPsValue>),
    List(Vec<JsPsValue>),
    Dictionary(Vec<JsDictionaryEntry>),
}

impl From<Container> for JsContainer {
    fn from(value: Container) -> Self {
        match value {
            Container::Stack(v) => Self::Stack(v.into_iter().map(JsPsValue::from).collect()),
            Container::Queue(v) => Self::Queue(v.into_iter().map(JsPsValue::from).collect()),
            Container::List(v) => Self::List(v.into_iter().map(JsPsValue::from).collect()),
            Container::Dictionary(map) => Self::Dictionary(
                map.into_iter()
                    .map(|(k, v)| JsDictionaryEntry {
                        key: JsPsValue::from(k),
                        value: JsPsValue::from(v),
                    })
                    .collect(),
            ),
        }
    }
}

impl TryFrom<JsContainer> for Container {
    type Error = String;

    fn try_from(value: JsContainer) -> Result<Self, Self::Error> {
        Ok(match value {
            JsContainer::Stack(v) => Self::Stack(
                v.into_iter()
                    .map(PsValue::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            JsContainer::Queue(v) => Self::Queue(
                v.into_iter()
                    .map(PsValue::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            JsContainer::List(v) => Self::List(
                v.into_iter()
                    .map(PsValue::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            JsContainer::Dictionary(entries) => {
                let mut out = BTreeMap::new();
                for entry in entries {
                    let k = PsValue::try_from(entry.key)?;
                    let v = PsValue::try_from(entry.value)?;
                    out.insert(k, v);
                }
                Self::Dictionary(out)
            }
        })
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPsEnums {
    pub value: i32,
}

impl From<PsEnums> for JsPsEnums {
    fn from(value: PsEnums) -> Self {
        Self { value: value.value }
    }
}

impl From<JsPsEnums> for PsEnums {
    fn from(value: JsPsEnums) -> Self {
        Self { value: value.value }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum JsComplexObjectContent {
    ExtendedPrimitive(JsPsPrimitiveValue),
    Container(JsContainer),
    PsEnums(JsPsEnums),
    Standard,
}

impl From<ComplexObjectContent> for JsComplexObjectContent {
    fn from(value: ComplexObjectContent) -> Self {
        match value {
            ComplexObjectContent::ExtendedPrimitive(p) => Self::ExtendedPrimitive(p.into()),
            ComplexObjectContent::Container(c) => Self::Container(c.into()),
            ComplexObjectContent::PsEnums(e) => Self::PsEnums(e.into()),
            ComplexObjectContent::Standard => Self::Standard,
        }
    }
}

impl TryFrom<JsComplexObjectContent> for ComplexObjectContent {
    type Error = String;

    fn try_from(value: JsComplexObjectContent) -> Result<Self, Self::Error> {
        Ok(match value {
            JsComplexObjectContent::ExtendedPrimitive(p) => {
                Self::ExtendedPrimitive(PsPrimitiveValue::try_from(p)?)
            }
            JsComplexObjectContent::Container(c) => Self::Container(Container::try_from(c)?),
            JsComplexObjectContent::PsEnums(e) => Self::PsEnums(e.into()),
            JsComplexObjectContent::Standard => Self::Standard,
        })
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsComplexObject {
    pub type_def: Option<JsPsType>,
    pub to_string: Option<String>,
    pub content: JsComplexObjectContent,
    pub adapted_properties: BTreeMap<String, JsPsProperty>,
    pub extended_properties: BTreeMap<String, JsPsProperty>,
}

impl From<ComplexObject> for JsComplexObject {
    fn from(value: ComplexObject) -> Self {
        Self {
            type_def: value.type_def.map(JsPsType::from),
            to_string: value.to_string,
            content: JsComplexObjectContent::from(value.content),
            adapted_properties: value
                .adapted_properties
                .into_iter()
                .map(|(k, v)| (k, JsPsProperty::from(v)))
                .collect(),
            extended_properties: value
                .extended_properties
                .into_iter()
                .map(|(k, v)| (k, JsPsProperty::from(v)))
                .collect(),
        }
    }
}

impl TryFrom<JsComplexObject> for ComplexObject {
    type Error = String;

    fn try_from(value: JsComplexObject) -> Result<Self, Self::Error> {
        Ok(Self {
            type_def: value.type_def.map(PsType::from),
            to_string: value.to_string,
            content: ComplexObjectContent::try_from(value.content)?,
            adapted_properties: value
                .adapted_properties
                .into_iter()
                .map(|(k, v)| Ok((k, PsProperty::try_from(v)?)))
                .collect::<Result<_, String>>()?,
            extended_properties: value
                .extended_properties
                .into_iter()
                .map(|(k, v)| Ok((k, PsProperty::try_from(v)?)))
                .collect::<Result<_, String>>()?,
        })
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct JsPushRunspaceStructured {
    pub runspace: JsPsValue,
}
