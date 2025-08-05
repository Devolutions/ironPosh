pub mod defragmenter;
pub mod fragment;
pub mod fragmenter;

#[cfg(test)]
mod tests;

pub use defragmenter::*;
pub use fragment::*;
pub use fragmenter::*;

/// Result of defragmentation operation
#[derive(Debug)]
pub enum DefragmentResult {
    /// No complete messages available yet, waiting for more fragments
    Incomplete,
    /// One or more complete messages have been assembled
    Complete(Vec<crate::PowerShellRemotingMessage>),
}
