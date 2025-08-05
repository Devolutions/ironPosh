use crate::PowerShellRemotingMessage;
use super::fragment::Fragment;

/// Fragmenter handles fragmentation of outgoing PowerShell remoting messages
pub struct Fragmenter {
    max_fragment_size: usize,
    outgoing_counter: u64,
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
}