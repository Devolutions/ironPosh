use tracing::{debug, trace};
use uuid::Uuid;

use super::fragment::Fragment;
use crate::{PowerShellRemotingError, PowerShellRemotingMessage, ps_value::PsObjectWithType};

/// Fragmenter handles fragmentation of outgoing PowerShell remoting messages
#[derive(Debug)]
pub struct Fragmenter {
    max_fragment_size: usize,
    outgoing_counter: u64,
}

fn safe_split_at(data: &[u8], size: usize) -> (&[u8], &[u8]) {
    if data.len() <= size {
        (data, &[])
    } else {
        data.split_at(size)
    }
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
    pub fn fragment(
        &mut self,
        ps_object: &dyn PsObjectWithType,
        rpid: Uuid,
        pid: Option<Uuid>,
        remaining_size: Option<usize>,
    ) -> Result<Vec<Vec<u8>>, PowerShellRemotingError> {
        let message = PowerShellRemotingMessage::from_ps_message(ps_object, rpid, pid)?;
        let message_bytes_source = message.pack();
        let mut remaining_bytes = message_bytes_source.as_slice();
        let max_size = self.max_fragment_size;
        let mut start = true;
        let mut fragment_id = 0;
        let mut fragments = Vec::new();

        if let Some(remaining_size) = remaining_size {
            let (frag1, remaining) = safe_split_at(remaining_bytes, remaining_size);
            let end = remaining.is_empty();

            remaining_bytes = remaining;

            let fragment = Fragment::new(
                self.outgoing_counter,
                fragment_id,
                frag1.to_vec(),
                start,
                end,
            );

            fragments.push(fragment.pack());
            fragment_id += 1;
            start = false;
        }

        for chunk in remaining_bytes.chunks(max_size) {
            let end = chunk.len() < max_size;
            let fragment = Fragment::new(
                self.outgoing_counter,
                fragment_id,
                chunk.to_vec(),
                start,
                end,
            );

            fragments.push(fragment.pack());
            fragment_id += 1;
            start = false;

            if end {
                break;
            }
        }

        self.outgoing_counter += 1;

        Ok(fragments)
    }

    /// Fragment multiple messages, grouping them by WSMAN request boundaries
    /// Returns a Vec where each inner Vec contains fragments that should be sent in one WSMAN request
    pub fn fragment_multiple(
        &mut self,
        messages: &[&dyn PsObjectWithType],
        rpid: Uuid,
        pid: Option<Uuid>,
    ) -> Result<Vec<Vec<u8>>, PowerShellRemotingError> {
        let mut remaing_size = self.max_fragment_size;
        // Here we perhaps should not call it fragments anymore
        // Because we are grouping multiple fragments together into one Vec<u8>
        let mut fragements: Vec<Vec<u8>> = Vec::new();

        for message in messages {
            let mut message_fragments = self.fragment(*message, rpid, pid, Some(remaing_size))?;
            trace!(
                "Fragmented message {:?} into {} fragments",
                message.message_type(),
                message_fragments.len()
            );

            // If we have remaining space, append the next message to the last fragment
            // This can save some space if the last fragment is not full
            if remaing_size != self.max_fragment_size && !fragements.is_empty() {
                debug!(
                    "Appending to last fragment, remaining size: {}",
                    remaing_size
                );
                if let Some(last) = fragements.last_mut() {
                    last.extend(message_fragments.remove(0));
                }
            }

            fragements.extend(message_fragments);

            remaing_size = self.max_fragment_size - fragements.last().map_or(0, Vec::len);
            if remaing_size == 0 {
                remaing_size = self.max_fragment_size; // Reset for next message
            }
        }

        Ok(fragements)
    }
}
