use std::collections::HashMap;
use byteorder::{BigEndian, WriteBytesExt};
use crate::{PowerShellRemotingMessage, PowerShellRemotingError};

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
        buffer.write_u32::<BigEndian>(self.data.len() as u32).unwrap();
        
        // Data payload
        buffer.extend_from_slice(&self.data);
        
        buffer
    }

    /// Unpack a fragment from wire format bytes
    pub fn unpack(data: &[u8]) -> Result<(Self, &[u8]), PowerShellRemotingError> {
        if data.len() < 21 {
            return Err(PowerShellRemotingError::InvalidMessage(
                "Fragment too short, need at least 21 bytes".to_string()
            ));
        }

        let object_id = u64::from_be_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        
        let fragment_id = u64::from_be_bytes([
            data[8], data[9], data[10], data[11],
            data[12], data[13], data[14], data[15],
        ]);
        
        let flags = data[16];
        let start = (flags & 0x01) != 0;
        let end = (flags & 0x02) != 0;
        
        let length = u32::from_be_bytes([data[17], data[18], data[19], data[20]]) as usize;
        
        if data.len() < 21 + length {
            return Err(PowerShellRemotingError::InvalidMessage(
                format!("Fragment data truncated: expected {} bytes, got {}", 21 + length, data.len())
            ));
        }
        
        let fragment_data = data[21..21 + length].to_vec();
        let remaining = &data[21 + length..];
        
        let fragment = Fragment::new(object_id, fragment_id, fragment_data, start, end);
        
        Ok((fragment, remaining))
    }
}

/// Buffer for accumulating fragments during defragmentation
#[derive(Debug)]
pub struct FragmentBuffer {
    data: Vec<u8>,
    expected_fragment_id: u64,
}

impl FragmentBuffer {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            expected_fragment_id: 0,
        }
    }
}

/// Fragmenter handles fragmentation and defragmentation of PowerShell remoting messages
pub struct Fragmenter {
    max_fragment_size: usize,
    outgoing_counter: u64,
    // Note: We don't store incoming buffers as fields - ownership is transferred to caller
}

impl Fragmenter {
    pub fn new(max_fragment_size: usize) -> Self {
        // Subtract header size (21 bytes) from max fragment size
        let actual_max_size = max_fragment_size.saturating_sub(21);
        
        Self {
            max_fragment_size: actual_max_size,
            outgoing_counter: 1,
        }
    }

    /// Fragment a single message into multiple fragments
    pub fn fragment(&mut self, message: &PowerShellRemotingMessage) -> Vec<Fragment> {
        let data = message.clone().into_vec();
        self.fragment_data(data)
    }

    /// Fragment multiple messages, grouping them by WSMAN request boundaries
    /// Returns a Vec where each inner Vec contains fragments that should be sent in one WSMAN request
    pub fn fragment_multiple(&mut self, messages: &[PowerShellRemotingMessage]) -> Vec<Vec<Fragment>> {
        let mut request_groups: Vec<Vec<Fragment>> = Vec::new();
        let mut current_request: Vec<Fragment> = Vec::new();
        let mut remaining_space = self.max_fragment_size;
        
        for message in messages {
            let message_data = message.clone().into_vec();
            let fragments = self.fragment_data_with_remaining_space(message_data, remaining_space);
            
            // If we can fit the first fragment with previous data in current request, merge them
            if !current_request.is_empty() && remaining_space < self.max_fragment_size && remaining_space > 0
                && let Some(first_fragment) = fragments.first()
                    && first_fragment.data.len() <= remaining_space {
                        // Merge with the last fragment in current request
                        if let Some(last_fragment) = current_request.last_mut() {
                            last_fragment.data.extend_from_slice(&first_fragment.data);
                            // Add remaining fragments (if any) to current request
                            current_request.extend(fragments.into_iter().skip(1));
                        }
                        // Calculate remaining space more accurately
                        if let Some(last_fragment) = current_request.last() {
                            remaining_space = self.max_fragment_size - (last_fragment.data.len() % self.max_fragment_size);
                            if remaining_space == self.max_fragment_size {
                                remaining_space = 0;
                            }
                        }
                        continue;
                    }
            
            // If current request would exceed reasonable size, start a new request
            if !current_request.is_empty() && (current_request.len() + fragments.len() > 10 || remaining_space == 0) {
                request_groups.push(std::mem::take(&mut current_request));
                remaining_space = self.max_fragment_size;
            }
            
            current_request.extend(fragments);
            
            // Calculate remaining space in the last fragment
            if let Some(last_fragment) = current_request.last() {
                remaining_space = self.max_fragment_size - (last_fragment.data.len() % self.max_fragment_size);
                if remaining_space == self.max_fragment_size {
                    remaining_space = 0; // Fragment is full
                }
            }
        }
        
        // Add the final request group if it has any fragments
        if !current_request.is_empty() {
            request_groups.push(current_request);
        }
        
        request_groups
    }

    /// Fragment raw data into fragments
    fn fragment_data(&mut self, data: Vec<u8>) -> Vec<Fragment> {
        self.fragment_data_with_remaining_space(data, self.max_fragment_size)
    }

    /// Fragment raw data with consideration for remaining space in previous fragment
    fn fragment_data_with_remaining_space(&mut self, data: Vec<u8>, remaining_space: usize) -> Vec<Fragment> {
        let mut fragments = Vec::new();
        let object_id = self.outgoing_counter;
        self.outgoing_counter += 1;
        
        if data.is_empty() {
            // Empty data still needs a fragment
            return vec![Fragment::new(object_id, 0, Vec::new(), true, true)];
        }
        
        let mut fragment_id = 0u64;
        let mut start = true;
        let mut remaining_data = data.as_slice();
        
        // Handle first fragment with remaining space consideration
        if remaining_space < self.max_fragment_size && !remaining_data.is_empty() {
            let chunk_size = remaining_space.min(remaining_data.len());
            let chunk = remaining_data[..chunk_size].to_vec();
            let end = chunk_size >= remaining_data.len();
            
            fragments.push(Fragment::new(object_id, fragment_id, chunk, start, end));
            
            if end {
                return fragments;
            }
            
            remaining_data = &remaining_data[chunk_size..];
            fragment_id += 1;
            start = false;
        }
        
        // Handle remaining data in full-sized chunks
        while !remaining_data.is_empty() {
            let chunk_size = self.max_fragment_size.min(remaining_data.len());
            let chunk = remaining_data[..chunk_size].to_vec();
            let end = chunk_size >= remaining_data.len();
            
            fragments.push(Fragment::new(object_id, fragment_id, chunk, start, end));
            
            remaining_data = &remaining_data[chunk_size..];
            fragment_id += 1;
            start = false;
        }
        
        fragments
    }

    /// Defragment multiple fragments back into complete messages
    /// Takes ownership of the incoming buffer and yields ownership back via the returned buffer
    pub fn defragment(
        &self, 
        data: Vec<u8>, 
        mut incoming_buffer: Option<HashMap<u64, FragmentBuffer>>
    ) -> Result<(Vec<PowerShellRemotingMessage>, HashMap<u64, FragmentBuffer>), PowerShellRemotingError> {
        
        // Take ownership or create new buffer
        let mut buffer = incoming_buffer.take().unwrap_or_default();
        let mut messages = Vec::new();
        let mut remaining_data = data.as_slice();
        
        // Parse all fragments from the data
        while !remaining_data.is_empty() {
            let (fragment, rest) = Fragment::unpack(remaining_data)?;
            remaining_data = rest;
            
            let object_id = fragment.object_id;
            
            // Get or create buffer for this object
            let fragment_buffer = buffer.entry(object_id).or_insert_with(FragmentBuffer::new);
            
            // Validate fragment sequence
            if fragment.fragment_id != fragment_buffer.expected_fragment_id {
                return Err(PowerShellRemotingError::InvalidMessage(
                    format!(
                        "Fragment sequence error for object {}: expected fragment {}, got {}",
                        object_id, fragment_buffer.expected_fragment_id, fragment.fragment_id
                    )
                ));
            }
            
            // Handle complete single fragment
            if fragment.start && fragment.end {
                let message = self.parse_message(fragment.data)?;
                messages.push(message);
                buffer.remove(&object_id);
                continue;
            }
            
            // Handle fragment start
            if fragment.start {
                fragment_buffer.data.clear();
                fragment_buffer.expected_fragment_id = 0;
            }
            
            // Accumulate fragment data
            fragment_buffer.data.extend_from_slice(&fragment.data);
            fragment_buffer.expected_fragment_id += 1;
            
            // Handle fragment end
            if fragment.end {
                let complete_data = std::mem::take(&mut fragment_buffer.data);
                let message = self.parse_message(complete_data)?;
                messages.push(message);
                buffer.remove(&object_id);
            }
        }
        
        // Return messages and transfer ownership of buffer back to caller
        Ok((messages, buffer))
    }

    /// Parse a complete message from reassembled data
    fn parse_message(&self, data: Vec<u8>) -> Result<PowerShellRemotingMessage, PowerShellRemotingError> {
        let mut cursor = std::io::Cursor::new(data);
        PowerShellRemotingMessage::parse(&mut cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Destination, MessageType};
    use uuid::Uuid;
    use crate::messages::PsObject;

    fn create_test_message(data_size: usize) -> PowerShellRemotingMessage {
        let large_data = vec![b'A'; data_size];
        let large_string = String::from_utf8(large_data).unwrap();
        
        let mut props = std::collections::HashMap::new();
        props.insert(crate::PsValue::Str("TestData".to_string()), crate::PsValue::Str(large_string));
        
        let ps_object = PsObject {
            ref_id: None,
            type_names: None,
            tn_ref: None,
            props: vec![],
            ms: vec![],
            lst: vec![],
            dct: props,
        };
        
        PowerShellRemotingMessage::new(
            Destination::Server,
            MessageType::SessionCapability,
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            &ps_object,
        )
    }

    #[test]
    fn test_fragment_single_message_small() {
        let mut fragmenter = Fragmenter::new(1000);
        let message = create_test_message(50);
        
        let fragments = fragmenter.fragment(&message);
        
        assert_eq!(fragments.len(), 1);
        assert!(fragments[0].start);
        assert!(fragments[0].end);
        assert_eq!(fragments[0].fragment_id, 0);
    }

    #[test]
    fn test_fragment_single_message_large() {
        let mut fragmenter = Fragmenter::new(100);
        let message = create_test_message(250);
        
        let fragments = fragmenter.fragment(&message);
        
        assert!(fragments.len() > 1);
        assert!(fragments[0].start);
        assert!(!fragments[0].end);
        assert!(fragments.last().unwrap().end);
        assert!(!fragments.last().unwrap().start);
    }

    #[test]
    fn test_fragment_multiple_messages() {
        let mut fragmenter = Fragmenter::new(200);
        let messages = vec![
            create_test_message(50),
            create_test_message(75),
            create_test_message(100),
        ];
        
        let request_groups = fragmenter.fragment_multiple(&messages);
        
        assert!(!request_groups.is_empty());
        assert!(!request_groups[0].is_empty());
        
        // Flatten all fragments to check overall structure
        let all_fragments: Vec<&Fragment> = request_groups.iter().flat_map(|group| group.iter()).collect();
        
        // First fragment should start
        assert!(all_fragments[0].start);
        // Last fragment should end
        assert!(all_fragments.last().unwrap().end);
        
        // Each request group should have reasonable boundaries
        for group in &request_groups {
            assert!(!group.is_empty());
            assert!(group.len() <= 10); // Our heuristic limit
        }
    }

    #[test]
    fn test_defragment_single_message() {
        let mut fragmenter = Fragmenter::new(100);
        let original_message = create_test_message(250);
        
        // Fragment the message
        let fragments = fragmenter.fragment(&original_message);
        
        // Pack fragments into wire format
        let mut wire_data = Vec::new();
        for fragment in fragments {
            wire_data.extend_from_slice(&fragment.pack());
        }
        
        // Defragment
        let (messages, _buffer) = fragmenter.defragment(wire_data, None).unwrap();
        
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].destination as u32, original_message.destination as u32);
        assert_eq!(messages[0].message_type.value(), original_message.message_type.value());
    }

    #[test]
    fn test_defragment_partial_fragments() {
        let mut fragmenter = Fragmenter::new(100);
        let message = create_test_message(250);
        
        let fragments = fragmenter.fragment(&message);
        assert!(fragments.len() > 1);
        
        // Send only first fragment
        let first_fragment_data = fragments[0].pack();
        let (messages, buffer) = fragmenter.defragment(first_fragment_data, None).unwrap();
        
        // Should have no complete messages yet
        assert_eq!(messages.len(), 0);
        assert_eq!(buffer.len(), 1);
        
        // Send remaining fragments
        let mut remaining_data = Vec::new();
        for fragment in &fragments[1..] {
            remaining_data.extend_from_slice(&fragment.pack());
        }
        
        let (messages, buffer) = fragmenter.defragment(remaining_data, Some(buffer)).unwrap();
        
        // Should now have complete message
        assert_eq!(messages.len(), 1);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_fragment_unpack_pack_roundtrip() {
        let original = Fragment::new(123, 456, vec![1, 2, 3, 4, 5], true, false);
        let packed = original.pack();
        let (unpacked, remaining) = Fragment::unpack(&packed).unwrap();
        
        assert_eq!(remaining.len(), 0);
        assert_eq!(unpacked.object_id, original.object_id);
        assert_eq!(unpacked.fragment_id, original.fragment_id);
        assert_eq!(unpacked.start, original.start);
        assert_eq!(unpacked.end, original.end);
        assert_eq!(unpacked.data, original.data);
    }
}