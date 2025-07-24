pub mod messages;
use std::io::Read;

use byteorder::ReadBytesExt;

#[derive(Debug, Clone)]
pub enum Destination {
    Client,
    Server,
}

impl Destination {
    pub fn value(&self) -> u32 {
        match self {
            Destination::Client => 0x00000001,
            Destination::Server => 0x00000002,
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
                "Unknown MessageType value: 0x{:08x}",
                value
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
    pub rpid: RPID,
    /// PowerShell Process ID (PID)
    pub pid: PID,
}

/// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-psrp/3610dae4-67f7-4175-82da-a3fab83af288
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct PowerShellFragment<'a> {
    pub object_id: [u8; 8],
    pub fragment_id: [u8; 8],
    pub end_of_fragment: bool,
    pub start_of_fragment: bool,
    pub blob_length: [u8; 4],
    pub blob: &'a [u8],
}

impl<'a> PowerShellFragment<'a> {
    pub fn parse(
        cursor: &mut std::io::Cursor<&'a [u8]>,
    ) -> Result<Self, crate::PowerShellRemotingError> {
        let mut object_id = [0u8; 8];
        let mut fragment_id = [0u8; 8];
        let mut blob_length = [0u8; 4];
        cursor.read_exact(&mut object_id)?;

        cursor.read_exact(&mut fragment_id)?;

        let flags = cursor.read_u8()?;

        let end_of_fragment = (flags & 0b0000_0010) != 0;
        let start_of_fragment = (flags & 0b0000_0001) != 0;

        cursor.read_exact(&mut blob_length)?;

        let blob_size = u32::from_be_bytes(blob_length) as usize;

        let current_pos = cursor.position() as usize;
        let data = cursor.get_ref();
        if current_pos + blob_size > data.len() {
            return Err(crate::PowerShellRemotingError::InvalidMessage(
                "Not enough data for PowerShell fragment".to_string(),
            ));
        }

        let blob = &cursor.get_ref()[current_pos..current_pos + blob_size];
        cursor.set_position((current_pos + blob_size) as u64);

        Ok(Self {
            object_id,
            fragment_id,
            end_of_fragment,
            start_of_fragment,
            blob_length,
            blob,
        })
    }
}

impl std::fmt::Display for PowerShellFragment<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PowerShellFragment {{ object_id: {:?}, fragment_id: {:?}, end_of_fragment: {}, start_of_fragment: {}, blob_length: {:?}, blob: [{}] }}",
            self.object_id
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" "),
            self.fragment_id
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" "),
            self.end_of_fragment,
            self.start_of_fragment,
            self.blob_length
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" "),
            self.blob
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}
