//! SimpleWriter — Ergonomic high-level API for writing MF4 files
//!
//! Provides a minimal-boilerplate interface for the common single-DG, single-CG case.
//!
//! # Example
//! ```ignore
//! use mf4_parse::writer::simple_writer::SimpleWriter;
//!
//! let mut writer = SimpleWriter::new("output.mf4")
//!     .author("My App")
//!     .time_channel("time", "s")
//!     .f64_channel("voltage", "V")
//!     .f64_channel("current", "A")
//!     .compression(6)
//!     .stream_mode()
//!     .build()?;
//!
//! writer.write_record(&[0.0, 3.14, 1.5])?;
//! writer.write_record(&[0.001, 2.72, 1.6])?;
//! writer.finalize()?;
//! ```

use std::io::BufWriter;
use std::path::{Path, PathBuf};

use super::builder::Mf4Metadata;
use super::error::{WriteError, WriteResult};
use super::stream_writer::{
    ChannelDef, ChannelGroupDefBuilder, Mf4StreamWriter, StreamingConfig, StreamingDataGroup,
};

/// Channel definition for SimpleWriter builder
struct SimpleChannel {
    name: String,
    unit: String,
    data_type: u8,
    bit_count: u32,
    is_master: bool,
}

/// Builder for constructing a [`SimpleWriter`] with fluent API
///
/// # Example
/// ```ignore
/// let mut writer = SimpleWriter::new("output.mf4")
///     .author("My App")
///     .time_channel("time", "s")
///     .f64_channel("voltage", "V")
///     .compression(6)
///     .build()?;
/// ```
pub struct SimpleWriterBuilder {
    path: PathBuf,
    author: String,
    comment: String,
    project: String,
    organization: String,
    group_name: String,
    channels: Vec<SimpleChannel>,
    compression_level: u8,
    compression_threshold: u64,
    compact: bool,
}

impl SimpleWriterBuilder {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            author: String::new(),
            comment: String::new(),
            project: String::new(),
            organization: String::new(),
            group_name: "data".to_string(),
            channels: Vec::new(),
            compression_level: 0,
            compression_threshold: 100_000,
            compact: false,
        }
    }

    /// Set the file author
    pub fn author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }

    /// Set the file comment
    pub fn comment(mut self, comment: &str) -> Self {
        self.comment = comment.to_string();
        self
    }

    /// Set the project name
    pub fn project(mut self, project: &str) -> Self {
        self.project = project.to_string();
        self
    }

    /// Set the organization name
    pub fn organization(mut self, org: &str) -> Self {
        self.organization = org.to_string();
        self
    }

    /// Set the channel group name
    pub fn group_name(mut self, name: &str) -> Self {
        self.group_name = name.to_string();
        self
    }

    /// Add a master time channel (f64, unit typically "s")
    pub fn time_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 4,
            bit_count: 64,
            is_master: true,
        });
        self
    }

    /// Add an f64 data channel
    pub fn f64_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 4,
            bit_count: 64,
            is_master: false,
        });
        self
    }

    /// Add an f32 data channel
    pub fn f32_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 4,
            bit_count: 32,
            is_master: false,
        });
        self
    }

    /// Add a u8 data channel
    pub fn u8_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 0,
            bit_count: 8,
            is_master: false,
        });
        self
    }

    /// Add a u16 data channel
    pub fn u16_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 0,
            bit_count: 16,
            is_master: false,
        });
        self
    }

    /// Add a u32 data channel
    pub fn u32_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 0,
            bit_count: 32,
            is_master: false,
        });
        self
    }

    /// Add a u64 data channel
    pub fn u64_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 0,
            bit_count: 64,
            is_master: false,
        });
        self
    }

    /// Add an i16 data channel
    pub fn i16_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 2,
            bit_count: 16,
            is_master: false,
        });
        self
    }

    /// Add an i32 data channel
    pub fn i32_channel(mut self, name: &str, unit: &str) -> Self {
        self.channels.push(SimpleChannel {
            name: name.to_string(),
            unit: unit.to_string(),
            data_type: 2,
            bit_count: 32,
            is_master: false,
        });
        self
    }

    /// Set compression level (0 = off, 1-9 = zlib level)
    pub fn compression(mut self, level: u8) -> Self {
        self.compression_level = level;
        self
    }

    /// Set the compression threshold in bytes (default 100KB)
    ///
    /// Data smaller than this threshold won't be compressed.
    /// Set to 0 to always compress.
    pub fn compression_threshold(mut self, threshold: u64) -> Self {
        self.compression_threshold = threshold;
        self
    }

    /// Use stream mode: data split into DL-chained blocks (default)
    pub fn stream_mode(mut self) -> Self {
        self.compact = false;
        self
    }

    /// Use compact mode: all data in a single block
    pub fn compact_mode(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Build the SimpleWriter, ready to accept records
    pub fn build(self) -> WriteResult<SimpleWriter> {
        if self.channels.is_empty() {
            return Err(WriteError::InvalidChannelConfig(
                "SimpleWriter requires at least one channel".to_string(),
            ));
        }

        // Build StreamingConfig — block_size only affects DT (uncompressed) blocks;
        // DZ block sizes are fixed at 4MB internally per the MDF4 protocol.
        let mut config = StreamingConfig::new()
            .with_block_size(4_000_000)
            .with_compression_threshold(self.compression_threshold);
        if self.compression_level > 0 {
            config = config.with_compression_level(self.compression_level);
        }

        // Build metadata
        let mut metadata = Mf4Metadata::new();
        if !self.author.is_empty() {
            metadata = metadata.with_author(&self.author);
        }
        if !self.comment.is_empty() {
            metadata = metadata.with_comment(&self.comment);
        }
        if !self.project.is_empty() {
            metadata = metadata.with_project(&self.project);
        }
        if !self.organization.is_empty() {
            metadata = metadata.with_organization(&self.organization);
        }

        // Build channel group
        let mut cg_builder = ChannelGroupDefBuilder::new().name(&self.group_name);

        let mut channel_names = Vec::with_capacity(self.channels.len());

        for ch in &self.channels {
            if ch.is_master {
                cg_builder = cg_builder.master(
                    ChannelDef::new_master(&ch.name)
                );
                // Override unit if not "s"
                // new_master sets unit="s" by default, keep it for now
            } else {
                cg_builder = cg_builder.channel(
                    ChannelDef::new(&ch.name)
                        .data_type(ch.data_type)
                        .bit_count(ch.bit_count)
                        .unit(&ch.unit),
                );
            }
            channel_names.push(ch.name.clone());
        }

        let cg = cg_builder.build()?;
        let channel_count = channel_names.len();

        // Build writer
        let mut writer = Mf4StreamWriter::with_config(self.path, metadata, config)?;
        writer.add_data_group(StreamingDataGroup::new(cg)?)?;
        writer.finalize_structure()?;

        Ok(SimpleWriter {
            inner: writer,
            channel_names,
            channel_count,
            compact: self.compact,
            records_written: 0,
        })
    }
}

/// High-level MF4 writer for the common single-channel-group case
///
/// Wraps [`Mf4StreamWriter`] with a minimal API: `write_record` + `finalize`.
///
/// # Example
/// ```ignore
/// let mut writer = SimpleWriter::new("output.mf4")
///     .time_channel("time", "s")
///     .f64_channel("voltage", "V")
///     .build()?;
///
/// writer.write_record(&[0.0, 3.14])?;
/// writer.finalize()?;
/// ```
pub struct SimpleWriter {
    inner: Mf4StreamWriter<BufWriter<std::fs::File>>,
    channel_names: Vec<String>,
    channel_count: usize,
    compact: bool,
    records_written: u64,
}

impl SimpleWriter {
    /// Start building a new SimpleWriter for the given file path
    pub fn new(path: impl AsRef<Path>) -> SimpleWriterBuilder {
        SimpleWriterBuilder::new(path.as_ref().to_path_buf())
    }

    /// Write a complete record with values in channel definition order
    ///
    /// Values must correspond to channels in the order they were added
    /// (master/time first, then data channels).
    pub fn write_record(&mut self, values: &[f64]) -> WriteResult<()> {
        if values.len() != self.channel_count {
            return Err(WriteError::InvalidChannelConfig(
                format!(
                    "Expected {} values but got {}",
                    self.channel_count,
                    values.len()
                ),
            ));
        }

        self.inner.start_record(0, 0)?;
        for (i, &val) in values.iter().enumerate() {
            self.inner.set_channel_value(&self.channel_names[i], val)?;
        }
        self.inner.flush_record()?;
        self.records_written += 1;
        Ok(())
    }

    /// Get the number of records written so far
    pub fn records_written(&self) -> u64 {
        self.records_written
    }

    /// Finalize the file (writes block chain and closes the file)
    pub fn finalize(mut self) -> WriteResult<()> {
        self.inner.finalize_with_compact(self.compact)
    }
}
