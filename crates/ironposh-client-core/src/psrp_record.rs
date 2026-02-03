use ironposh_psrp::{InformationRecord, MessageType, ProgressRecord};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsrpRecordMeta {
    pub message_type: MessageType,
    pub message_type_value: u32,
    pub stream: String,
    pub command_id: Option<Uuid>,
    pub data_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PsrpRecord {
    Debug {
        meta: PsrpRecordMeta,
        message: String,
    },
    Verbose {
        meta: PsrpRecordMeta,
        message: String,
    },
    Warning {
        meta: PsrpRecordMeta,
        message: String,
    },
    Information {
        meta: PsrpRecordMeta,
        record: InformationRecord,
    },
    Progress {
        meta: PsrpRecordMeta,
        record: ProgressRecord,
    },
    Unsupported {
        meta: PsrpRecordMeta,
        data_preview: String,
    },
}
