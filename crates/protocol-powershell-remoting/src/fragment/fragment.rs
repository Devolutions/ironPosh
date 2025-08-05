use crate::PowerShellRemotingError;
use base64::Engine;
use byteorder::{BigEndian, WriteBytesExt};

/// Fragment represents a single fragment of a PowerShell remoting message
#[derive(Debug, Clone)]
pub struct Fragment {
    pub object_id: u64,
    pub fragment_id: u64,
    pub start: bool,
    pub end: bool,
    pub data: Vec<u8>,
}

impl Fragment {
    pub fn new(object_id: u64, fragment_id: u64, data: Vec<u8>, start: bool, end: bool) -> Self {
        Self {
            object_id,
            fragment_id,
            start,
            end,
            data,
        }
    }

    pub fn encode_multiple(fragments: &[Self]) -> Result<String, PowerShellRemotingError> {
        let mut encoded_fragments = Vec::new();
        for fragment in fragments {
            encoded_fragments.push(fragment.pack_as_base64());
        }
        Ok(encoded_fragments.join(""))
    }

    /// Pack the fragment into wire format bytes
    pub fn pack(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Object ID (8 bytes, big endian)
        buffer.write_u64::<BigEndian>(self.object_id).unwrap();

        // Fragment ID (8 bytes, big endian)
        buffer.write_u64::<BigEndian>(self.fragment_id).unwrap();

        // Start/End flags (1 byte)
        let mut flags = 0u8;
        if self.start {
            flags |= 0x01;
        }
        if self.end {
            flags |= 0x02;
        }
        buffer.push(flags);

        // Data length (4 bytes, big endian)
        buffer
            .write_u32::<BigEndian>(self.data.len() as u32)
            .unwrap();

        // Data payload
        buffer.extend_from_slice(&self.data);

        buffer
    }

    pub fn pack_as_base64(&self) -> String {
        let packed = self.pack();
        base64::engine::general_purpose::STANDARD.encode(packed)
    }

    /// Unpack a fragment from wire format bytes
    pub fn unpack(data: &[u8]) -> Result<(Self, &[u8]), PowerShellRemotingError> {
        if data.len() < 21 {
            return Err(PowerShellRemotingError::InvalidMessage(
                "Fragment too short, need at least 21 bytes".to_string(),
            ));
        }

        let object_id = u64::from_be_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);

        let fragment_id = u64::from_be_bytes([
            data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
        ]);

        let flags = data[16];
        let start = (flags & 0x01) != 0;
        let end = (flags & 0x02) != 0;

        let length = u32::from_be_bytes([data[17], data[18], data[19], data[20]]) as usize;

        if data.len() < 21 + length {
            return Err(PowerShellRemotingError::InvalidMessage(format!(
                "Fragment data truncated: expected {} bytes, got {}",
                21 + length,
                data.len()
            )));
        }

        let fragment_data = data[21..21 + length].to_vec();
        let remaining = &data[21 + length..];

        let fragment = Fragment::new(object_id, fragment_id, fragment_data, start, end);

        Ok((fragment, remaining))
    }
}
