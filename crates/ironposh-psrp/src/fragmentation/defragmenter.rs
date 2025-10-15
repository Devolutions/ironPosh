use tracing::trace;

use super::{DefragmentResult, fragment::Fragment};
use crate::{PowerShellRemotingError, PowerShellRemotingMessage};
use std::collections::HashMap;

/// Buffer for accumulating fragments during defragmentation
#[derive(Debug)]
struct FragmentBuffer {
    fragments: Vec<Fragment>,
    is_complete: bool,
}

impl FragmentBuffer {
    fn new() -> Self {
        Self {
            fragments: Vec::new(),
            is_complete: false,
        }
    }

    /// Add a fragment to this buffer if it's the expected next fragment
    fn add_fragment(&mut self, fragment: Fragment) {
        if fragment.end {
            self.is_complete = true;
        }
        self.fragments.push(fragment);
    }

    /// Reassemble all fragments into complete message data
    fn reassemble(&self) -> Vec<u8> {
        let mut frags = self.fragments.clone();
        frags.sort_by(|a, b| a.fragment_id.cmp(&b.fragment_id));
        let total_len: usize = frags.iter().map(|f| f.data.len()).sum();
        let mut out = Vec::with_capacity(total_len);

        for f in frags {
            out.extend_from_slice(&f.data);
        }
        out
    }
}

/// Defragmenter handles defragmentation of incoming PowerShell remoting message fragments
/// with internal state management
#[derive(Debug, Default)]
pub struct Defragmenter {
    buffers: HashMap<u64, FragmentBuffer>,
}

impl Defragmenter {
    /// Create a new defragmenter
    pub fn new() -> Self {
        Self::default()
    }

    /// Process incoming packet data containing one or more fragments
    /// Returns complete messages if any are ready, or Incomplete if still waiting
    pub fn defragment(
        &mut self,
        packet_data: &[u8],
    ) -> Result<DefragmentResult, PowerShellRemotingError> {
        let mut remaining_data = packet_data;
        let mut completed_messages = Vec::new();

        // Parse all fragments from the packet data
        while !remaining_data.is_empty() {
            let (fragment, rest) = Fragment::unpack(remaining_data)?;
            trace!(
                fragment = ?fragment,
                "Defragmenter unpacked fragment"
            );

            remaining_data = rest;
            trace!(
                remaining_data_len = remaining_data.len(),
                "Remaining data after unpacking fragment"
            );

            let object_id = fragment.object_id;

            // Handle complete single-fragment message
            if fragment.start && fragment.end {
                let message = Self::parse_message(fragment.data)?;
                completed_messages.push(message);
                continue;
            }

            // Get or create buffer for this object
            let buffer = self
                .buffers
                .entry(object_id)
                .or_insert_with(FragmentBuffer::new);

            // Handle start fragment - reset buffer
            if fragment.start {
                *buffer = FragmentBuffer::new();
            }

            // Add fragment to buffer
            buffer.add_fragment(fragment);

            // Check if message is complete
            if buffer.is_complete {
                let complete_data = buffer.reassemble();
                let message = Self::parse_message(complete_data)?;
                completed_messages.push(message);
                self.buffers.remove(&object_id);
            }
        }

        if completed_messages.is_empty() {
            Ok(DefragmentResult::Incomplete)
        } else {
            Ok(DefragmentResult::Complete(completed_messages))
        }
    }

    /// Get the number of incomplete message buffers
    pub fn pending_count(&self) -> usize {
        self.buffers.len()
    }

    /// Clear all incomplete buffers (useful for error recovery)
    pub fn clear_buffers(&mut self) {
        self.buffers.clear();
    }

    /// Parse a complete message from reassembled data
    fn parse_message(data: Vec<u8>) -> Result<PowerShellRemotingMessage, PowerShellRemotingError> {
        let mut cursor = std::io::Cursor::new(data);
        PowerShellRemotingMessage::parse(&mut cursor)
    }
}
