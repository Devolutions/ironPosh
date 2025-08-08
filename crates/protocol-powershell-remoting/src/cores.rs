use std::io::Read;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use uuid::Uuid;

use crate::{PsObjectWithType, PsValue};

#[derive(Debug, Clone, Copy)]
pub enum Destination {
    Client = 0x0000_0001,
    Server = 0x0000_0002,
}

impl TryFrom<u32> for Destination {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x0000_0001 => Ok(Destination::Client),
            0x0000_0002 => Ok(Destination::Server),
            _ => Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Unknown Destination value: 0x{value:08x}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageType {
    SessionCapability,
    InitRunspacepool,
    PublicKey,
    EncryptedSessionKey,
    PublicKeyRequest,
    ConnectRunspacepool,
    RunspacepoolInitData,
    ResetRunspaceState,
    SetMaxRunspaces,
    SetMinRunspaces,
    RunspaceAvailability,
    RunspacepoolState,
    CreatePipeline,
    GetAvailableRunspaces,
    UserEvent,
    ApplicationPrivateData,
    GetCommandMetadata,
    RunspacepoolHostCall,
    RunspacepoolHostResponse,
    PipelineInput,
    EndOfPipelineInput,
    PipelineOutput,
    ErrorRecord,
    PipelineState,
    DebugRecord,
    VerboseRecord,
    WarningRecord,
    ProgressRecord,
    InformationRecord,
    PipelineHostCall,
    PipelineHostResponse,
}

impl MessageType {
    pub fn value(&self) -> u32 {
        match self {
            MessageType::SessionCapability => 0x00010002,
            MessageType::InitRunspacepool => 0x00010004,
            MessageType::PublicKey => 0x00010005,
            MessageType::EncryptedSessionKey => 0x00010006,
            MessageType::PublicKeyRequest => 0x00010007,
            MessageType::ConnectRunspacepool => 0x00010008,
            MessageType::RunspacepoolInitData => 0x0002100B,
            MessageType::ResetRunspaceState => 0x0002100C,
            MessageType::SetMaxRunspaces => 0x00021002,
            MessageType::SetMinRunspaces => 0x00021003,
            MessageType::RunspaceAvailability => 0x00021004,
            MessageType::RunspacepoolState => 0x00021005,
            MessageType::CreatePipeline => 0x00021006,
            MessageType::GetAvailableRunspaces => 0x00021007,
            MessageType::UserEvent => 0x00021008,
            MessageType::ApplicationPrivateData => 0x00021009,
            MessageType::GetCommandMetadata => 0x0002100A,
            MessageType::RunspacepoolHostCall => 0x00021100,
            MessageType::RunspacepoolHostResponse => 0x00021101,
            MessageType::PipelineInput => 0x00041002,
            MessageType::EndOfPipelineInput => 0x00041003,
            MessageType::PipelineOutput => 0x00041004,
            MessageType::ErrorRecord => 0x00041005,
            MessageType::PipelineState => 0x00041006,
            MessageType::DebugRecord => 0x00041007,
            MessageType::VerboseRecord => 0x00041008,
            MessageType::WarningRecord => 0x00041009,
            MessageType::ProgressRecord => 0x00041010,
            MessageType::InformationRecord => 0x00041011,
            MessageType::PipelineHostCall => 0x00041100,
            MessageType::PipelineHostResponse => 0x00041101,
        }
    }
}

impl TryFrom<u32> for MessageType {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x00010002 => Ok(MessageType::SessionCapability),
            0x00010004 => Ok(MessageType::InitRunspacepool),
            0x00010005 => Ok(MessageType::PublicKey),
            0x00010006 => Ok(MessageType::EncryptedSessionKey),
            0x00010007 => Ok(MessageType::PublicKeyRequest),
            0x00010008 => Ok(MessageType::ConnectRunspacepool),
            0x0002100B => Ok(MessageType::RunspacepoolInitData),
            0x0002100C => Ok(MessageType::ResetRunspaceState),
            0x00021002 => Ok(MessageType::SetMaxRunspaces),
            0x00021003 => Ok(MessageType::SetMinRunspaces),
            0x00021004 => Ok(MessageType::RunspaceAvailability),
            0x00021005 => Ok(MessageType::RunspacepoolState),
            0x00021006 => Ok(MessageType::CreatePipeline),
            0x00021007 => Ok(MessageType::GetAvailableRunspaces),
            0x00021008 => Ok(MessageType::UserEvent),
            0x00021009 => Ok(MessageType::ApplicationPrivateData),
            0x0002100A => Ok(MessageType::GetCommandMetadata),
            0x00021100 => Ok(MessageType::RunspacepoolHostCall),
            0x00021101 => Ok(MessageType::RunspacepoolHostResponse),
            0x00041002 => Ok(MessageType::PipelineInput),
            0x00041003 => Ok(MessageType::EndOfPipelineInput),
            0x00041004 => Ok(MessageType::PipelineOutput),
            0x00041005 => Ok(MessageType::ErrorRecord),
            0x00041006 => Ok(MessageType::PipelineState),
            0x00041007 => Ok(MessageType::DebugRecord),
            0x00041008 => Ok(MessageType::VerboseRecord),
            0x00041009 => Ok(MessageType::WarningRecord),
            0x00041010 => Ok(MessageType::ProgressRecord),
            0x00041011 => Ok(MessageType::InformationRecord),
            0x00041100 => Ok(MessageType::PipelineHostCall),
            0x00041101 => Ok(MessageType::PipelineHostResponse),
            _ => Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Unknown MessageType value: 0x{value:08x}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RPID {
    pub value: [u8; 16],
}

#[derive(Debug, Clone)]
pub struct PID {
    pub value: [u8; 16],
}

#[derive(Debug, Clone)]
///https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/497ac440-89fb-4cb3-9cc1-3434c1aa74c3
pub struct PowerShellRemotingMessage {
    pub destination: Destination,
    pub message_type: MessageType,
    /// Runspace Pool ID (RPID)
    pub rpid: Uuid,
    /// PowerShell Process ID (PID)
    pub pid: Option<Uuid>,
    pub data: Vec<u8>, // This will hold the serialized PsObject data
}

impl PowerShellRemotingMessage {
    pub fn parse<T>(cursor: &mut std::io::Cursor<T>) -> Result<Self, crate::PowerShellRemotingError>
    where
        T: AsRef<[u8]>,
    {
        let destination = cursor.read_u32::<LittleEndian>()?;
        let message_type = cursor
            .read_u32::<LittleEndian>()
            .map(MessageType::try_from)??;

        let mut rpid_bytes = [0u8; 16];
        cursor.read_exact(&mut rpid_bytes)?;

        let mut pid_bytes = [0u8; 16];
        cursor.read_exact(&mut pid_bytes)?;

        let mut rest = vec![];
        cursor.read_to_end(&mut rest)?;

        Ok(Self {
            destination: Destination::try_from(destination).map_err(|e| {
                crate::PowerShellRemotingError::InvalidMessage(format!(
                    "Invalid destination value: {e}"
                ))
            })?,
            message_type,
            rpid: Uuid::from_bytes(rpid_bytes),
            pid: pid_bytes
                .iter()
                .all(|&b| b == 0)
                .then_some(Uuid::from_bytes(pid_bytes)),
            data: rest,
        })
    }

    pub fn from_ps_message(
        message: &dyn PsObjectWithType,
        rpid: Uuid,
        pid: Option<Uuid>,
    ) -> Result<PowerShellRemotingMessage, crate::PowerShellRemotingError> {
        let message_type = message.message_type();
        let data = message.to_ps_object();

        Self::new(Destination::Server, message_type, rpid, pid, &data)
    }

    pub fn new(
        destination: Destination,
        message_type: MessageType,
        rpid: Uuid,
        pid: Option<Uuid>,
        data: &PsValue,
    ) -> Result<Self, crate::PowerShellRemotingError> {
        Ok(Self {
            destination,
            message_type,
            rpid,
            pid,
            data: data.to_element_as_root()?.to_string().into_bytes(),
        })
    }

    pub fn pack(self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer
            .write_u32::<LittleEndian>(self.destination as u32)
            .unwrap();
        buffer
            .write_u32::<LittleEndian>(self.message_type.value())
            .unwrap();
        buffer.extend_from_slice(self.rpid.as_bytes());
        buffer.extend_from_slice(self.pid.unwrap_or_default().as_bytes());
        buffer.extend_from_slice(&self.data);
        buffer
    }
}

/// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/3610dae4-67f7-4175-82da-a3fab83af288
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct PowerShellFragment {
    pub object_id: u64,
    pub fragment_id: u64,
    pub end_of_fragment: bool,
    pub start_of_fragment: bool,
    pub blob_length: u32,
    pub blob: Vec<u8>, // This will hold the serialized PsObject data
}

impl PowerShellFragment {
    pub fn parse(
        cursor: &mut std::io::Cursor<&[u8]>,
    ) -> Result<Self, crate::PowerShellRemotingError> {
        let object_id = cursor.read_u64::<BigEndian>()?;
        let fragment_id = cursor.read_u64::<BigEndian>()?;
        let flags = cursor.read_u8()?;
        let end_of_fragment = (flags & 0b0000_0010) != 0;
        let start_of_fragment = (flags & 0b0000_0001) != 0;

        let blob_length = cursor.read_u32::<byteorder::BigEndian>()?;

        let current_pos = cursor.position() as usize;
        let data = cursor.get_ref();
        if current_pos + (blob_length as usize) > data.len() {
            return Err(crate::PowerShellRemotingError::InvalidMessage(format!(
                "Not enough data for PowerShell fragment at position {}: expected {}, got {}",
                current_pos,
                blob_length,
                data.len() - current_pos
            )));
        }

        let blob = &cursor.get_ref()[current_pos..current_pos + blob_length as usize];
        cursor.set_position(current_pos as u64 + blob_length as u64);

        Ok(Self {
            object_id,
            fragment_id,
            end_of_fragment,
            start_of_fragment,
            blob_length,
            blob: blob.to_vec(),
        })
    }

    pub fn new(
        object_id: u64,
        fragment_id: u64,
        end_of_fragment: bool,
        start_of_fragment: bool,
        blob: PowerShellRemotingMessage,
    ) -> Self {
        let blob_length = blob.data.len() as u32;
        Self {
            object_id,
            fragment_id,
            end_of_fragment,
            start_of_fragment,
            blob_length,
            blob: blob.pack(),
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.write_u64::<BigEndian>(self.object_id).unwrap();
        buffer.write_u64::<BigEndian>(self.fragment_id).unwrap();
        let flags = (self.end_of_fragment as u8) << 1 | (self.start_of_fragment as u8);
        buffer.write_u8(flags).unwrap();
        buffer.write_u32::<BigEndian>(self.blob_length).unwrap();
        buffer.extend_from_slice(&self.blob);
        buffer
    }
}

impl std::fmt::Display for PowerShellFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PowerShellFragment {{ object_id: {:?}, fragment_id: {:?}, end_of_fragment: {}, start_of_fragment: {}, blob_length: {:?}, blob: [{}] }}",
            self.object_id,
            self.fragment_id,
            self.end_of_fragment,
            self.start_of_fragment,
            self.blob_length,
            self.blob
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}
