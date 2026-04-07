//! Write error types for MF4 write operations

use std::fmt;
use std::io;

/// Write error types
#[derive(Debug)]
pub enum WriteError {
    /// I/O error
    IoError(io::Error),

    /// Invalid data type
    InvalidDataType {
        channel: String,
        expected: u8,
        actual: u8,
    },

    /// Data length mismatch
    DataLengthMismatch {
        channel: String,
        expected: usize,
        actual: usize,
    },

    /// Invalid block offset
    InvalidOffset { block: String, offset: u64 },

    /// Structure already finalized
    AlreadyFinalized,

    /// Structure not finalized
    NotFinalized,

    /// Channel not found
    ChannelNotFound { name: String },

    /// Compression error
    CompressionError(String),

    /// Invalid state for operation
    InvalidState { current: String, required: String },

    /// Block serialization error
    SerializationError(String),

    /// Missing required field
    MissingField(String),

    /// Invalid channel configuration
    InvalidChannelConfig(String),

    /// Unsupported feature
    UnsupportedFeature(String),
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::IoError(e) => write!(f, "I/O error: {}", e),
            WriteError::InvalidDataType { channel, expected, actual } => {
                write!(f, "Invalid data type for channel '{}': expected {}, got {}", channel, expected, actual)
            }
            WriteError::DataLengthMismatch { channel, expected, actual } => {
                write!(f, "Data length mismatch for channel '{}': expected {} samples, got {}", channel, expected, actual)
            }
            WriteError::InvalidOffset { block, offset } => {
                write!(f, "Invalid block offset for {}: {:#X}", block, offset)
            }
            WriteError::AlreadyFinalized => write!(f, "Structure already finalized, cannot modify"),
            WriteError::NotFinalized => write!(f, "Structure not finalized, call finalize_structure() first"),
            WriteError::ChannelNotFound { name } => write!(f, "Channel not found: {}", name),
            WriteError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            WriteError::InvalidState { current, required } => {
                write!(f, "Invalid state: current={}, required={}", current, required)
            }
            WriteError::SerializationError(msg) => write!(f, "Block serialization error: {}", msg),
            WriteError::MissingField(field) => write!(f, "Missing required field: {}", field),
            WriteError::InvalidChannelConfig(msg) => write!(f, "Invalid channel configuration: {}", msg),
            WriteError::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
        }
    }
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::IoError(e)
    }
}

/// Result type for write operations
pub type WriteResult<T> = Result<T, WriteError>;
