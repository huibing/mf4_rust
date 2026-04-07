//! MF4 Write Feature
//!
//! This module provides functionality to write MF4 files. It supports two modes:
//! - **One-time Write**: Create complete MF4 files in a single operation using `Mf4Builder`
//! - **Streaming Write**: Incrementally append data using `Mf4StreamWriter`

#[cfg(feature = "write")]
pub mod builder;

#[cfg(feature = "streaming")]
pub mod stream_writer;

#[cfg(feature = "write")]
pub mod block_writer;

#[cfg(feature = "compression")]
pub mod compression;

#[cfg(feature = "write")]
pub mod error;

#[cfg(test)]
mod write_test;

// Re-export main types for convenience
#[cfg(feature = "write")]
pub use builder::{Mf4Builder, Mf4Metadata, CompressionConfig, DataGroupBuilder, ChannelGroupBuilder, ChannelBuilder, ConversionBuilder, SourceInfoBuilder, SourceType, BusType};

#[cfg(feature = "streaming")]
pub use stream_writer::{Mf4StreamWriter, StreamingDataGroup, ChannelGroupDef, ChannelDef, WriterState, StreamingConfig};

#[cfg(feature = "write")]
pub use error::WriteError;
