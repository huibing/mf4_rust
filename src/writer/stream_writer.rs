//! Mf4StreamWriter - Streaming API for incremental MF4 file creation
//!
//! This module provides streaming write capability for applications that need to
//! incrementally append data to MF4 files over time.
//!
//! # Architecture
//!
//! Streaming writes use a **chained data block** strategy:
//! - Data is written in fixed-size blocks (configurable)
//! - Multiple DT/DZ blocks are linked via DL block
//! - On finalize, optionally compact into a single block
//!
//! ```text
//! DG.dg_data ──> DL Block ──> DT Block 1 (block_size)
//!                        ├──> DT Block 2
//!                        └──> DT Block 3
//! ```

use std::path::PathBuf;
use std::io::{BufWriter, Write, Seek};

use super::error::{WriteError, WriteResult};
use super::builder::{Mf4Metadata, SourceInfoBuilder, ConversionParams};

// ============================================================================
// Streaming Configuration
// ============================================================================

/// Configuration for streaming write
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Size of uncompressed DT data blocks in bytes (default: 1MB).
    /// This setting has no effect on DZ (compressed) blocks, whose size is fixed
    /// at the 4MB MDF4 protocol maximum and managed internally by the library.
    pub block_size: u64,
    /// Enable compression for data blocks
    pub enable_compression: bool,
    /// Compression threshold (data size above this will be compressed)
    pub compression_threshold: u64,
    /// Compression level (1-9)
    pub compression_level: u8,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            block_size: 1_000_000, // 1 MB
            enable_compression: false,
            compression_threshold: 100_000, // 100 KB
            compression_level: 6,
        }
    }
}

impl StreamingConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the block size for uncompressed DT data blocks.
    ///
    /// This setting does **not** affect DZ (compressed) block sizes. DZ block
    /// sizes are fixed at the MDF4 protocol maximum of 4MB and are managed
    /// internally by the library.
    pub fn with_block_size(mut self, size: u64) -> Self {
        self.block_size = size;
        self
    }

    /// Enable compression with default settings
    pub fn with_compression(mut self) -> Self {
        self.enable_compression = true;
        self
    }

    /// Enable compression with custom settings
    pub fn with_compression_level(mut self, level: u8) -> Self {
        self.enable_compression = true;
        self.compression_level = level.clamp(1, 9);
        self
    }

    /// Set the compression threshold (data below this size won't be compressed)
    pub fn with_compression_threshold(mut self, threshold: u64) -> Self {
        self.compression_threshold = threshold;
        self
    }
}

// ============================================================================
// Writer State
// ============================================================================

/// Writer state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterState {
    /// File created, metadata written
    Initialized,
    /// Channel structure defined, ready for data
    StructureReady,
    /// Data being appended
    Writing,
    /// File finalized and closed
    Finalized,
}

impl Default for WriterState {
    fn default() -> Self {
        Self::Initialized
    }
}

impl std::fmt::Display for WriterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriterState::Initialized => write!(f, "Initialized"),
            WriterState::StructureReady => write!(f, "StructureReady"),
            WriterState::Writing => write!(f, "Writing"),
            WriterState::Finalized => write!(f, "Finalized"),
        }
    }
}

// ============================================================================
// Channel Definition (Immutable after creation)
// ============================================================================

/// Definition of a channel (immutable after creation)
#[derive(Debug, Clone)]
pub struct ChannelDef {
    /// Channel name
    pub name: String,
    /// Data type (0-10)
    pub data_type: u8,
    /// Byte offset within record
    pub byte_offset: u32,
    /// Bit offset within byte
    pub bit_offset: u8,
    /// Number of bits
    pub bit_count: u32,
    /// Unit string
    pub unit: Option<String>,
    /// Channel type (Fixed, VLSD, Master, VirtualMaster)
    pub cn_type: u8,
    /// Sync type (None, Time, Angle, Distance, Index)
    pub sync_type: u8,
    /// Optional conversion (e.g. Value2Text / vtab)
    pub conversion: Option<ConversionParams>,
}

impl ChannelDef {
    /// Create a new channel definition
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: 4, // FLOAT64 LE
            byte_offset: 0,
            bit_offset: 0,
            bit_count: 64,
            unit: None,
            cn_type: 0,
            sync_type: 0,
            conversion: None,
        }
    }

    /// Create a new master time channel
    pub fn new_master(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: 4, // FLOAT64 LE
            byte_offset: 0,
            bit_offset: 0,
            bit_count: 64,
            unit: Some("s".to_string()),
            cn_type: 2, // Master
            sync_type: 1, // Time
            conversion: None,
        }
    }

    /// Set the data type
    pub fn data_type(mut self, data_type: u8) -> Self {
        self.data_type = data_type;
        self
    }

    /// Set the number of bits
    pub fn bit_count(mut self, bit_count: u32) -> Self {
        self.bit_count = bit_count;
        self
    }

    /// Set the unit
    pub fn unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    /// Attach a Value-to-Text (vtab) conversion (CC type 7).
    ///
    /// `keys` are the raw numeric values stored in the data section.
    /// `texts` are the corresponding display strings (one per key).
    /// `default` is the fallback text for unmatched raw values.
    ///
    /// # Example
    /// ```ignore
    /// ChannelDef::new("gear")
    ///     .data_type(0).bit_count(8)
    ///     .vtab(vec![1.0, 2.0, 3.0], vec!["1st".into(), "2nd".into(), "3rd".into()], "N/A".into())
    /// ```
    pub fn vtab(mut self, keys: Vec<f64>, texts: Vec<String>, default: String) -> Self {
        self.conversion = Some(ConversionParams::Value2Text { keys, texts, default });
        self
    }

    /// Attach a Value-Range-to-Text conversion (CC type 8).
    ///
    /// `ranges` are `(min, max)` inclusive bounds for each entry.
    /// `texts` are the corresponding display strings (one per range).
    /// `default` is the fallback text for values that match no range.
    ///
    /// # Example
    /// ```ignore
    /// ChannelDef::new("temp_band")
    ///     .data_type(4).bit_count(64)
    ///     .vrange(vec![(0.0, 50.0), (50.0, 100.0)], vec!["Cold".into(), "Hot".into()], "OOB".into())
    /// ```
    pub fn vrange(mut self, ranges: Vec<(f64, f64)>, texts: Vec<String>, default: String) -> Self {
        self.conversion = Some(ConversionParams::ValueRange2Text { ranges, texts, default });
        self
    }

    /// Get the number of bytes for this channel
    pub fn bytes_count(&self) -> u32 {
        (self.bit_count + 7) / 8
    }
}

// ============================================================================
// Channel Group Definition (Immutable after creation)
// ============================================================================

/// Definition of a channel group (immutable after creation)
#[derive(Debug, Clone)]
pub struct ChannelGroupDef {
    /// Acquisition name
    pub acq_name: String,
    /// Acquisition source (SI block info)
    pub acq_source: Option<SourceInfoBuilder>,
    /// Channel definitions
    pub channels: Vec<ChannelDef>,
    /// Master channel definition
    pub master: Option<ChannelDef>,
    /// Record ID (for multi-CG scenarios)
    pub record_id: u64,
    /// Record size in bytes
    pub record_size: u32,
    /// Data bytes (excluding invalid bytes)
    pub data_bytes: u32,
    /// Channel name → index lookup cache for O(1) access
    channel_index: std::collections::HashMap<String, ChannelLookup>,
}

/// Cached channel lookup result to avoid repeated linear scans
#[derive(Debug, Clone)]
enum ChannelLookup {
    /// Index into `channels` vec
    Regular(usize),
    /// Master channel
    Master,
}

impl ChannelGroupDef {
    /// Create a new channel group definition builder
    pub fn builder() -> ChannelGroupDefBuilder {
        ChannelGroupDefBuilder::new()
    }

    /// Get total cycle count written so far
    pub fn cycle_count(&self) -> u64 {
        // This will be tracked separately in StreamingDataGroup
        0
    }

    /// Get channel by name (O(1) via cached index)
    pub fn get_channel(&self, name: &str) -> Option<&ChannelDef> {
        match self.channel_index.get(name)? {
            ChannelLookup::Regular(idx) => self.channels.get(*idx),
            ChannelLookup::Master => self.master.as_ref(),
        }
    }
}

/// Builder for ChannelGroupDef
#[derive(Debug, Clone)]
pub struct ChannelGroupDefBuilder {
    acq_name: String,
    acq_source: Option<SourceInfoBuilder>,
    channels: Vec<ChannelDef>,
    master: Option<ChannelDef>,
    record_id: u64,
}

impl ChannelGroupDefBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            acq_name: String::new(),
            acq_source: None,
            channels: Vec::new(),
            master: None,
            record_id: 0,
        }
    }

    /// Set the acquisition name
    pub fn name(mut self, name: &str) -> Self {
        self.acq_name = name.to_string();
        self
    }

    /// Set the acquisition source (SI block)
    pub fn acq_source(mut self, source: SourceInfoBuilder) -> Self {
        self.acq_source = Some(source);
        self
    }

    /// Set the record ID
    pub fn record_id(mut self, record_id: u64) -> Self {
        self.record_id = record_id;
        self
    }

    /// Set the master channel
    pub fn master(mut self, channel: ChannelDef) -> Self {
        self.master = Some(channel);
        self
    }

    /// Add a channel
    pub fn channel(mut self, channel: ChannelDef) -> Self {
        self.channels.push(channel);
        self
    }

    /// Set master time channel (shorthand for `.master(ChannelDef::new_master(name))`)
    pub fn with_time_channel(self, name: &str) -> Self {
        self.master(ChannelDef::new_master(name))
    }

    /// Add an f64 data channel (FLOAT64 LE, 64-bit)
    pub fn add_f64_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(4).bit_count(64).unit(unit))
    }

    /// Add an f32 data channel (FLOAT32 LE, 32-bit)
    pub fn add_f32_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(4).bit_count(32).unit(unit))
    }

    /// Add a u8 data channel (UINT LE, 8-bit)
    pub fn add_u8_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(0).bit_count(8).unit(unit))
    }

    /// Add a u16 data channel (UINT LE, 16-bit)
    pub fn add_u16_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(0).bit_count(16).unit(unit))
    }

    /// Add a u32 data channel (UINT LE, 32-bit)
    pub fn add_u32_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(0).bit_count(32).unit(unit))
    }

    /// Add a u64 data channel (UINT LE, 64-bit)
    pub fn add_u64_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(0).bit_count(64).unit(unit))
    }

    /// Add an i8 data channel (INT LE, 8-bit)
    pub fn add_i8_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(2).bit_count(8).unit(unit))
    }

    /// Add an i16 data channel (INT LE, 16-bit)
    pub fn add_i16_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(2).bit_count(16).unit(unit))
    }

    /// Add an i32 data channel (INT LE, 32-bit)
    pub fn add_i32_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(2).bit_count(32).unit(unit))
    }

    /// Add an i64 data channel (INT LE, 64-bit)
    pub fn add_i64_channel(self, name: &str, unit: &str) -> Self {
        self.channel(ChannelDef::new(name).data_type(2).bit_count(64).unit(unit))
    }

    /// Add a Value-to-Text (vtab, CC type 7) channel.
    ///
    /// Raw numeric values (`data_type`, `bit_count`) are stored in the data section.
    /// The CC block maps each key to the corresponding display text; unmatched values
    /// fall back to `default`.
    ///
    /// # Example
    /// ```ignore
    /// cg_builder.add_vtab_channel(
    ///     "gear", 0, 8,
    ///     vec![1.0, 2.0, 3.0],
    ///     vec!["1st".into(), "2nd".into(), "3rd".into()],
    ///     "N/A",
    /// )
    /// ```
    pub fn add_vtab_channel(
        self,
        name: &str,
        data_type: u8,
        bit_count: u32,
        keys: Vec<f64>,
        texts: Vec<String>,
        default: &str,
    ) -> Self {
        self.channel(
            ChannelDef::new(name)
                .data_type(data_type)
                .bit_count(bit_count)
                .vtab(keys, texts, default.to_string()),
        )
    }

    /// Add a channel with a Value-Range-to-Text (CC type 8) conversion.
    ///
    /// Raw numeric values are stored in the data section; the CC block maps
    /// each `[min, max]` range to a display string. Values outside all ranges
    /// fall back to `default`.
    pub fn add_vrange_channel(
        self,
        name: &str,
        data_type: u8,
        bit_count: u32,
        ranges: Vec<(f64, f64)>,
        texts: Vec<String>,
        default: &str,
    ) -> Self {
        self.channel(
            ChannelDef::new(name)
                .data_type(data_type)
                .bit_count(bit_count)
                .vrange(ranges, texts, default.to_string()),
        )
    }

    /// Build the channel group definition
    pub fn build(self) -> WriteResult<ChannelGroupDef> {
        if self.channels.is_empty() && self.master.is_none() {
            return Err(WriteError::InvalidChannelConfig(
                "Channel group must have at least one channel".to_string(),
            ));
        }

        // Calculate byte offsets
        let mut current_offset: u32 = 0;
        let mut channels = self.channels;

        // Master comes first if present
        let mut master = self.master;
        if let Some(ref mut m) = master {
            m.byte_offset = current_offset;
            current_offset += m.bytes_count();
        }

        // Then regular channels
        for ch in channels.iter_mut() {
            ch.byte_offset = current_offset;
            current_offset += ch.bytes_count();
        }

        // Build channel name → index lookup
        let mut channel_index = std::collections::HashMap::with_capacity(
            channels.len() + if master.is_some() { 1 } else { 0 }
        );
        if let Some(ref m) = master {
            channel_index.insert(m.name.clone(), ChannelLookup::Master);
        }
        for (i, ch) in channels.iter().enumerate() {
            channel_index.insert(ch.name.clone(), ChannelLookup::Regular(i));
        }

        Ok(ChannelGroupDef {
            acq_name: self.acq_name,
            acq_source: self.acq_source,
            channels,
            master,
            record_id: self.record_id,
            record_size: current_offset,
            data_bytes: current_offset,
            channel_index,
        })
    }
}

impl Default for ChannelGroupDefBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Record Data
// ============================================================================

/// Record data for streaming write
#[derive(Debug, Clone)]
pub struct RecordData {
    /// Record ID (for multi-CG scenarios)
    pub record_id: u64,
    /// Raw record bytes
    pub data: Vec<u8>,
}

impl RecordData {
    /// Create a new record with the given size
    pub fn new(record_id: u64, size: usize) -> Self {
        Self {
            record_id,
            data: vec![0u8; size],
        }
    }

    /// Write a value at the specified offset
    pub fn write_u8(&mut self, offset: usize, value: u8) {
        if offset < self.data.len() {
            self.data[offset] = value;
        }
    }

    pub fn write_u16_le(&mut self, offset: usize, value: u16) {
        if offset + 2 <= self.data.len() {
            self.data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
    }

    pub fn write_u32_le(&mut self, offset: usize, value: u32) {
        if offset + 4 <= self.data.len() {
            self.data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    pub fn write_u64_le(&mut self, offset: usize, value: u64) {
        if offset + 8 <= self.data.len() {
            self.data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
    }

    pub fn write_f32_le(&mut self, offset: usize, value: f32) {
        if offset + 4 <= self.data.len() {
            self.data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    pub fn write_f64_le(&mut self, offset: usize, value: f64) {
        if offset + 8 <= self.data.len() {
            self.data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
}

// ============================================================================
// Streaming Data Group
// ============================================================================

/// Streaming data group that supports incremental writes
#[derive(Debug)]
pub struct StreamingDataGroup {
    /// Channel group definition (fixed after creation)
    pub channel_groups: Vec<ChannelGroupDef>,
    /// Record ID size (0, 1, 2, 4, 8 bytes)
    pub rec_id_size: u8,
    /// Current cycle count per channel group
    pub cycle_counts: Vec<u64>,
    /// Data block chain for managing multiple DT/DZ blocks
    pub data_chain: DataBlockChain,
    /// Shared data buffer. Records from all channel groups are appended here
    /// in the order they are flushed (chronological), so the DT block is
    /// already time-sorted as required by the MDF4 specification.
    pub shared_buffer: Vec<u8>,
    /// Pending record being built
    pub pending_record: Option<(usize, RecordData)>,
    /// Reusable record buffer to avoid per-record allocation
    reusable_record: Option<RecordData>,
    /// File offset of this DG block (set during finalize_structure)
    pub dg_offset: Option<u64>,
    /// File offset of data area (set during finalize_structure)
    pub data_area_offset: Option<u64>,
}

impl StreamingDataGroup {
    /// Create a new streaming data group with a single channel group
    pub fn new(cg: ChannelGroupDef) -> WriteResult<Self> {
        Self::with_config(cg, StreamingConfig::default())
    }

    /// Create a new streaming data group with configuration
    pub fn with_config(mut cg: ChannelGroupDef, config: StreamingConfig) -> WriteResult<Self> {
        // Ensure record_id is at least 1 (MDF specification requires record_id >= 1)
        if cg.record_id == 0 {
            cg.record_id = 1;
        }
        // rec_id_size is 0 for single-CG, so effective record size = record_size
        let effective_record_size = cg.record_size;
        // Pre-allocate shared_buffer to block_size to reduce reallocations
        let initial_capacity = config.block_size as usize;
        Ok(Self {
            channel_groups: vec![cg],
            rec_id_size: 0,
            cycle_counts: vec![0],
            shared_buffer: Vec::with_capacity(initial_capacity),
            data_chain: DataBlockChain::with_record_size(config, effective_record_size),
            pending_record: None,
            reusable_record: None,
            dg_offset: None,
            data_area_offset: None,
        })
    }

    /// Create a new streaming data group with multiple channel groups
    pub fn with_multiple(cgs: Vec<ChannelGroupDef>) -> WriteResult<Self> {
        Self::with_multiple_config(cgs, StreamingConfig::default())
    }

    /// Create a new streaming data group with multiple channel groups and configuration
    pub fn with_multiple_config(mut cgs: Vec<ChannelGroupDef>, config: StreamingConfig) -> WriteResult<Self> {
        if cgs.is_empty() {
            return Err(WriteError::MissingField("channel_groups".to_string()));
        }

        let rec_id_size = if cgs.len() == 1 {
            0
        } else if cgs.len() <= 255 {
            1
        } else if cgs.len() <= 65535 {
            2
        } else {
            4
        };

        // Assign unique record IDs to each channel group (1, 2, 3, ...)
        for (i, cg) in cgs.iter_mut().enumerate() {
            cg.record_id = (i + 1) as u64;
        }

        // For multi-CG, use the largest effective record size for alignment.
        // Each record includes rec_id_size prefix bytes.
        let max_record_size = cgs.iter()
            .map(|cg| cg.record_size + rec_id_size as u32)
            .max()
            .unwrap_or(0);

        let initial_capacity = config.block_size as usize;
        Ok(Self {
            cycle_counts: vec![0; cgs.len()],
            shared_buffer: Vec::with_capacity(initial_capacity),
            channel_groups: cgs,
            rec_id_size,
            data_chain: DataBlockChain::with_record_size(config, max_record_size),
            pending_record: None,
            reusable_record: None,
            dg_offset: None,
            data_area_offset: None,
        })
    }

    /// Get the total cycle count
    pub fn total_cycle_count(&self) -> u64 {
        self.cycle_counts.iter().sum()
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.data_chain.buffer_size()
    }

    /// Check if the current block is full and needs to be flushed
    pub fn is_block_full(&self) -> bool {
        self.data_chain.is_buffer_full()
    }

    /// Start a new record for the given channel group index
    pub fn start_record(&mut self, cg_index: usize) -> WriteResult<()> {
        if cg_index >= self.channel_groups.len() {
            return Err(WriteError::InvalidChannelConfig(format!(
                "Invalid channel group index: {}",
                cg_index
            )));
        }

        let cg = &self.channel_groups[cg_index];
        let record_size = cg.record_size as usize + self.rec_id_size as usize;

        // Reuse previously allocated record buffer if same size, otherwise allocate
        let mut record = match self.reusable_record.take() {
            Some(mut r) if r.data.len() == record_size => {
                // Zero out and reuse
                r.record_id = cg.record_id;
                r.data.iter_mut().for_each(|b| *b = 0);
                r
            }
            _ => RecordData::new(cg.record_id, record_size),
        };

        // Write record ID if needed
        if self.rec_id_size > 0 {
            match self.rec_id_size {
                1 => record.write_u8(0, cg.record_id as u8),
                2 => record.write_u16_le(0, cg.record_id as u16),
                4 => record.write_u32_le(0, cg.record_id as u32),
                8 => record.write_u64_le(0, cg.record_id),
                _ => {}
            }
        }

        self.pending_record = Some((cg_index, record));
        Ok(())
    }

    /// Set a channel value in the pending record
    pub fn set_channel_value<T: RecordValue>(&mut self, channel_name: &str, value: T) -> WriteResult<()> {
        let (cg_index, record) = self.pending_record.as_mut()
            .ok_or(WriteError::InvalidState {
                current: "No pending record".to_string(),
                required: "Pending record started".to_string(),
            })?;

        let cg = &self.channel_groups[*cg_index];
        let channel = cg.get_channel(channel_name)
            .ok_or_else(|| WriteError::ChannelNotFound { name: channel_name.to_string() })?;

        let offset = channel.byte_offset as usize + self.rec_id_size as usize;
        value.write_to_record(record, offset, channel.data_type, channel.bit_count);

        Ok(())
    }

    /// Complete and flush the current record
    pub fn flush_record(&mut self) -> WriteResult<()> {
        let (cg_index, record) = self.pending_record.take()
            .ok_or(WriteError::InvalidState {
                current: "No pending record".to_string(),
                required: "Pending record started".to_string(),
            })?;

        self.shared_buffer.extend_from_slice(&record.data);
        self.cycle_counts[cg_index] += 1;
        // Save record buffer for reuse in next start_record
        self.reusable_record = Some(record);
        Ok(())
    }
}

// ============================================================================
// Record Value Trait
// ============================================================================

/// Trait for writing values to records
pub trait RecordValue {
    /// Write the value to the record at the given offset
    fn write_to_record(&self, record: &mut RecordData, offset: usize, data_type: u8, bit_count: u32);
}

impl RecordValue for f64 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, data_type: u8, bit_count: u32) {
        match (data_type, bit_count) {
            // IEEE 754 float (LE/BE both written LE here — BE swapped at record level if needed)
            (4 | 5, 32) => record.write_f32_le(offset, *self as f32),
            (4 | 5, 16) => {
                let h = half::f16::from_f64(*self);
                record.write_u16_le(offset, h.to_bits());
            }
            // Unsigned integers
            (0 | 1,  8) => record.write_u8(offset, *self as u8),
            (0 | 1, 16) => record.write_u16_le(offset, *self as u16),
            (0 | 1, 32) => record.write_u32_le(offset, *self as u32),
            (0 | 1, 64) => record.write_u64_le(offset, *self as u64),
            // Signed integers
            (2 | 3,  8) => record.write_u8(offset, (*self as i8) as u8),
            (2 | 3, 16) => record.write_u16_le(offset, (*self as i16) as u16),
            (2 | 3, 32) => record.write_u32_le(offset, (*self as i32) as u32),
            (2 | 3, 64) => record.write_u64_le(offset, (*self as i64) as u64),
            // f64 (data_type 4/5, 64-bit) and any unrecognised type
            _ => record.write_f64_le(offset, *self),
        }
    }
}

impl RecordValue for f32 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_f32_le(offset, *self);
    }
}

impl RecordValue for u8 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u8(offset, *self);
    }
}

impl RecordValue for u16 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u16_le(offset, *self);
    }
}

impl RecordValue for u32 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u32_le(offset, *self);
    }
}

impl RecordValue for u64 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u64_le(offset, *self);
    }
}

impl RecordValue for i8 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u8(offset, *self as u8);
    }
}

impl RecordValue for i16 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u16_le(offset, *self as u16);
    }
}

impl RecordValue for i32 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u32_le(offset, *self as u32);
    }
}

impl RecordValue for i64 {
    fn write_to_record(&self, record: &mut RecordData, offset: usize, _data_type: u8, _bit_count: u32) {
        record.write_u64_le(offset, *self as u64);
    }
}

// ============================================================================
// Data Block Chain (Chained DT/DZ blocks via DL)
// ============================================================================

/// Maximum original (uncompressed) length for a single DZ block: 4 MiB.
///
/// This is a **mandatory ASAM MDF4 protocol requirement**, not a user-configurable option.
/// The library enforces this limit internally on every DZ block it writes.
pub const MAX_DZ_UNCOMPRESSED_SIZE: u64 = 4 * 1024 * 1024;

/// Represents a single data block in the chain
#[derive(Debug, Clone)]
pub struct DataBlockInfo {
    /// File offset of this block
    pub offset: u64,
    /// Size of this block (including header)
    pub size: u64,
    /// Whether this block is compressed
    pub compressed: bool,
}

/// Manages a chain of data blocks linked via DL block
///
/// This structure implements the chained data block strategy:
/// - Data is written in fixed-size blocks (record-aligned)
/// - When a block is full, a new one is created
/// - All blocks are linked via a DL block
/// - For compressed blocks, an HL block wraps the DL
/// - On finalize, optionally compact into a single block
///
/// Block hierarchy produced:
/// - Uncompressed: DG → DL → [DT₁, DT₂, ..., DTₙ]
/// - Compressed:   DG → HL → DL → [DZ₁, DZ₂, ..., DZₙ]
#[derive(Debug)]
pub struct DataBlockChain {
    /// Configuration
    config: StreamingConfig,
    /// Current write buffer (accumulating records)
    current_buffer: Vec<u8>,
    /// List of completed blocks (written to file)
    blocks: Vec<DataBlockInfo>,
    /// File offset where DL block will be written (set during finalize_structure)
    dl_block_offset: Option<u64>,
    /// Total bytes written
    total_bytes: u64,
    /// Record size in bytes (record_size + rec_id_size) for alignment.
    /// When 0, no alignment is enforced.
    record_size: u32,
}

impl DataBlockChain {
    /// Create a new data block chain
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            current_buffer: Vec::new(),
            blocks: Vec::new(),
            dl_block_offset: None,
            total_bytes: 0,
            record_size: 0,
        }
    }

    /// Create a new data block chain with record size for alignment
    pub fn with_record_size(config: StreamingConfig, record_size: u32) -> Self {
        Self {
            config,
            current_buffer: Vec::new(),
            blocks: Vec::new(),
            dl_block_offset: None,
            total_bytes: 0,
            record_size,
        }
    }

    /// Set the record size for alignment
    pub fn set_record_size(&mut self, record_size: u32) {
        self.record_size = record_size;
    }

    /// Returns the effective data chunk size for deciding when to flush a block.
    ///
    /// - **Compressed (DZ):** always `MAX_DZ_UNCOMPRESSED_SIZE` (4MB). The MDF4 protocol
    ///   forbids DZ blocks with an original length > 4MB. This is a hard internal limit,
    ///   not user-configurable.
    /// - **Uncompressed (DT):** the user-configured `block_size`.
    ///
    /// Both values are aligned down to the nearest record boundary when `record_size > 0`.
    fn effective_block_size(&self) -> u64 {
        // DZ blocks are bounded by the MDF4 protocol (4MB max). DT blocks use block_size.
        let base = if self.config.enable_compression {
            MAX_DZ_UNCOMPRESSED_SIZE
        } else {
            self.config.block_size
        };

        // Align down to record boundary
        if self.record_size > 0 {
            let rs = self.record_size as u64;
            (base / rs) * rs
        } else {
            base
        }
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.current_buffer.len()
    }

    /// Check if the current buffer is full
    pub fn is_buffer_full(&self) -> bool {
        self.current_buffer.len() as u64 >= self.effective_block_size()
    }

    /// Append record data to the current buffer
    pub fn append(&mut self, data: &[u8]) {
        self.current_buffer.extend_from_slice(data);
    }

    /// Get the number of blocks in the chain
    pub fn block_count(&self) -> usize {
        self.blocks.len() + if self.current_buffer.is_empty() { 0 } else { 1 }
    }

    /// Get total data bytes
    pub fn total_data_bytes(&self) -> u64 {
        self.total_bytes + self.current_buffer.len() as u64
    }

    /// Set the DL block offset (where the DL block will be written)
    pub fn set_dl_offset(&mut self, offset: u64) {
        self.dl_block_offset = Some(offset);
    }

    /// Get the DL block offset
    pub fn dl_offset(&self) -> Option<u64> {
        self.dl_block_offset
    }

    /// Check if any blocks in the chain are compressed
    pub fn has_compressed_blocks(&self) -> bool {
        self.blocks.iter().any(|b| b.compressed)
    }

    /// Finalize the current buffer as a new block
    /// Returns the offset of the written block
    pub fn finalize_current_block<W: Write + Seek>(&mut self, writer: &mut BlockWriter<W>) -> WriteResult<Option<u64>> {
        if self.current_buffer.is_empty() {
            return Ok(None);
        }

        let data = std::mem::take(&mut self.current_buffer);
        let data_len = data.len() as u64;

        // Decide whether to compress
        let should_compress = self.config.enable_compression
            && data_len >= self.config.compression_threshold;

        let offset = if should_compress {
            let compressor = super::compression::Compressor {
                compression_type: super::compression::CompressionType::Deflate,
                level: self.config.compression_level,
                column_count: None,
            };
            let (compressed_data, original_len) = compressor.compress(&data)?;
            let dz = super::block_writer::DzBlock {
                dz_org_data_length: original_len,
                dz_data_length: compressed_data.len() as u64,
                dz_zip_type: 0, // Deflate
                dz_zip_parameter: 0,
                data: compressed_data,
            };
            writer.write_dz_block(&dz)?
        } else {
            writer.write_dt_block(&super::block_writer::DtBlock::new(data))?
        };

        self.blocks.push(DataBlockInfo {
            offset,
            size: 0,
            compressed: should_compress,
        });

        self.total_bytes += data_len;
        Ok(Some(offset))
    }

    /// Split data into record-aligned chunks and write each as a DT or DZ block.
    /// Then create DL block (and HL block if compressed).
    /// Returns the top-level offset (HL for compressed, DL for uncompressed).
    pub fn write_chunked_chain<W: Write + Seek>(
        &mut self,
        writer: &mut BlockWriter<W>,
        data: Vec<u8>,
    ) -> WriteResult<u64> {
        let effective_size = self.effective_block_size() as usize;
        let total_data_len = data.len() as u64;

        // Compute record-aligned chunk boundaries (start, end) without copying
        let chunk_ranges = if self.record_size > 0 && effective_size > 0 {
            self.compute_record_aligned_ranges(data.len(), effective_size)
        } else if effective_size > 0 {
            let mut ranges = Vec::new();
            let mut offset = 0;
            while offset < data.len() {
                let end = (offset + effective_size).min(data.len());
                ranges.push((offset, end));
                offset = end;
            }
            ranges
        } else {
            vec![(0, data.len())]
        };

        // Reset blocks for this chain
        self.blocks.clear();
        self.total_bytes = 0;

        // Decide compression once based on total data size, not per-chunk
        let should_compress = self.config.enable_compression
            && total_data_len >= self.config.compression_threshold;
        let mut chunk_sizes: Vec<u64> = Vec::with_capacity(chunk_ranges.len());

        // Write each chunk as DT or DZ — reference slices, no copy
        for (start, end) in &chunk_ranges {
            let chunk = &data[*start..*end];
            let chunk_len = chunk.len() as u64;
            chunk_sizes.push(chunk_len);

            if should_compress {
                let compressor = super::compression::Compressor {
                    compression_type: super::compression::CompressionType::Deflate,
                    level: self.config.compression_level,
                    column_count: None,
                };
                let (compressed_data, original_len) = compressor.compress(chunk)?;
                let dz = super::block_writer::DzBlock {
                    dz_org_data_length: original_len,
                    dz_data_length: compressed_data.len() as u64,
                    dz_zip_type: 0,
                    dz_zip_parameter: 0,
                    data: compressed_data,
                };
                let off = writer.write_dz_block(&dz)?;
                self.blocks.push(DataBlockInfo {
                    offset: off,
                    size: 0,
                    compressed: true,
                });
            } else {
                let off = writer.write_dt_block(&super::block_writer::DtBlock::new(chunk.to_vec()))?;
                self.blocks.push(DataBlockInfo {
                    offset: off,
                    size: 0,
                    compressed: false,
                });
            };

            self.total_bytes += chunk_len;
        }

        // Compute cumulative byte offsets within the concatenated uncompressed data
        let mut data_offsets: Vec<u64> = Vec::with_capacity(chunk_sizes.len());
        let mut cumulative = 0u64;
        for sz in &chunk_sizes {
            data_offsets.push(cumulative);
            cumulative += sz;
        }

        // Write DL block linking all data blocks
        let links: Vec<u64> = self.blocks.iter().map(|b| b.offset).collect();
        let dl_offset = writer.write_dl_block(&links, &data_offsets)?;

        // If any blocks are compressed, wrap with HL
        if self.has_compressed_blocks() {
            let hl = super::block_writer::HlBlock {
                hl_dl_first: dl_offset,
                hl_flags: 0,
                hl_zip_type: 0, // Deflate
            };
            let hl_offset = writer.write_hl_block(&hl)?;
            Ok(hl_offset)
        } else {
            Ok(dl_offset)
        }
    }

    /// Split data into record-aligned chunks of at most `max_size` bytes.
    /// Compute record-aligned chunk ranges (start, end) without data copying
    fn compute_record_aligned_ranges(&self, data_len: usize, max_size: usize) -> Vec<(usize, usize)> {
        let rs = self.record_size as usize;
        if rs == 0 || data_len == 0 {
            return vec![(0, data_len)];
        }

        let mut ranges = Vec::new();
        let mut offset = 0;

        while offset < data_len {
            let remaining = data_len - offset;
            let chunk_size = if remaining <= max_size {
                remaining
            } else {
                // Round down to record boundary
                (max_size / rs) * rs
            };

            if chunk_size == 0 {
                // Record is larger than max_size — take one full record
                let single_record = remaining.min(rs);
                ranges.push((offset, offset + single_record));
                offset += single_record;
            } else {
                ranges.push((offset, offset + chunk_size));
                offset += chunk_size;
            }
        }

        ranges
    }

    /// Write all blocks and create DL block
    /// Returns the DL block offset
    pub fn finalize_chain<W: Write + Seek>(&mut self, writer: &mut BlockWriter<W>) -> WriteResult<u64> {
        // Finalize any remaining data
        self.finalize_current_block(writer)?;

        // Create DL block linking all data blocks
        let links: Vec<u64> = self.blocks.iter().map(|b| b.offset).collect();
        // Compute cumulative data offsets (we don't have sizes tracked, use 0s as placeholder)
        let data_offsets: Vec<u64> = vec![0u64; links.len()];
        let dl_offset = writer.write_dl_block(&links, &data_offsets)?;

        Ok(dl_offset)
    }

    /// Compact all blocks into a single block (or a DL-chained series of DZ blocks
    /// when data exceeds 4MB and compression is enabled).
    /// Returns the top-level block offset.
    pub fn compact<W: Write + Seek>(&mut self, writer: &mut BlockWriter<W>, all_data: Vec<u8>) -> WriteResult<u64> {
        let data_len = all_data.len() as u64;

        // Decide whether to compress
        let should_compress = self.config.enable_compression
            && data_len >= self.config.compression_threshold;

        let offset = if should_compress {
            if data_len > MAX_DZ_UNCOMPRESSED_SIZE {
                // MDF4 forbids DZ blocks with original_length > 4MB.
                // Fall back to a DL-chained series of DZ blocks.
                self.write_chunked_chain(writer, all_data)?
            } else {
                let compressor = super::compression::Compressor {
                    compression_type: super::compression::CompressionType::Deflate,
                    level: self.config.compression_level,
                    column_count: None,
                };
                let (compressed_data, original_len) = compressor.compress(&all_data)?;
                let dz = super::block_writer::DzBlock {
                    dz_org_data_length: original_len,
                    dz_data_length: compressed_data.len() as u64,
                    dz_zip_type: 0, // Deflate
                    dz_zip_parameter: 0,
                    data: compressed_data,
                };
                writer.write_dz_block(&dz)?
            }
        } else {
            writer.write_dt_block(&super::block_writer::DtBlock::new(all_data))?
        };

        // Reset chain with single block
        self.blocks = vec![DataBlockInfo {
            offset,
            size: 0,
            compressed: should_compress,
        }];
        self.dl_block_offset = None; // No DL needed for single block

        Ok(offset)
    }
}

// Import BlockWriter for DataBlockChain
use super::block_writer::BlockWriter;

// ============================================================================
// Record sorting helpers
// ============================================================================

/// Sort a multi-CG DT block's records by their master-channel time value.
///
/// The MDF4 specification requires that all data records within a DG are stored
/// in ascending order of the master channel value (ASAM MDF4 §5.4). This function
/// performs a stable sort so that records already in order are left unchanged.
///
/// Assumptions (valid for all channels created by this library):
/// - Master channel has `byte_offset == 0` inside the record data (after rec_id bytes)
/// - Master channel is encoded as IEEE 754 f64 little-endian (data_type 4, bit_count 64)
fn sort_records_by_time(data: Vec<u8>, dg: &StreamingDataGroup) -> Vec<u8> {
    if data.is_empty() {
        return data;
    }

    // Build a lookup: rec_id -> (total_record_bytes, master_time_byte_offset_in_record)
    let mut rec_map: std::collections::HashMap<u64, (usize, usize)> =
        std::collections::HashMap::new();
    for cg in &dg.channel_groups {
        let master_offset = cg.master.as_ref().map_or(0, |m| m.byte_offset as usize);
        let record_total = dg.rec_id_size as usize + cg.record_size as usize;
        rec_map.insert(cg.record_id, (record_total, master_offset));
    }

    // First pass: collect (time, start, end) for each record
    let mut records: Vec<(f64, usize, usize)> = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let rec_id = match dg.rec_id_size {
            1 => data[offset] as u64,
            2 => {
                let b: [u8; 2] = data[offset..offset + 2].try_into().unwrap_or([0; 2]);
                u16::from_le_bytes(b) as u64
            }
            4 => {
                let b: [u8; 4] = data[offset..offset + 4].try_into().unwrap_or([0; 4]);
                u32::from_le_bytes(b) as u64
            }
            8 => {
                let b: [u8; 8] = data[offset..offset + 8].try_into().unwrap_or([0; 8]);
                u64::from_le_bytes(b)
            }
            _ => break,
        };

        let Some(&(record_total, master_offset)) = rec_map.get(&rec_id) else {
            break; // Unknown rec_id — stop parsing
        };

        let time_start = offset + dg.rec_id_size as usize + master_offset;
        let time = if time_start + 8 <= data.len() {
            let b: [u8; 8] = data[time_start..time_start + 8].try_into().unwrap_or([0; 8]);
            f64::from_le_bytes(b)
        } else {
            break;
        };

        records.push((time, offset, offset + record_total));
        offset += record_total;
    }

    // Stable sort by time so equal-time records keep their relative order
    records.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Second pass: write sorted records
    let mut sorted = Vec::with_capacity(data.len());
    for (_, start, end) in records {
        sorted.extend_from_slice(&data[start..end]);
    }
    sorted
}

// ============================================================================
// Mf4StreamWriter
// ============================================================================

/// Streaming writer for incremental data append
///
/// # Example
///
/// ```ignore
/// use mf4_parse::writer::{Mf4StreamWriter, Mf4Metadata, StreamingConfig, ChannelGroupDef, ChannelDef};
///
/// // Create with custom configuration
/// let config = StreamingConfig::new()
///     .with_block_size(10_000_000)  // 10 MB blocks
///     .with_compression();          // Enable compression
///
/// let mut writer = Mf4StreamWriter::with_config(
///     "output.mf4".into(),
///     Mf4Metadata::default(),
///     config
/// )?;
///
/// // Define channels
/// let time_def = ChannelDef::new_master("time");
/// let temp_def = ChannelDef::new("Temperature").data_type(4);
///
/// let cg_def = ChannelGroupDef::builder()
///     .name("Measurement")
///     .master(time_def)
///     .channel(temp_def)
///     .build()?;
///
/// let dg = StreamingDataGroup::new(cg_def)?;
/// writer.add_data_group(dg)?;
/// writer.finalize_structure()?;
///
/// // Write data
/// for i in 0..100 {
///     writer.start_record(0, 0)?;
///     writer.set_channel_value("time", i as f64 * 0.01)?;
///     writer.set_channel_value("Temperature", 20.0 + i as f64)?;
///     writer.flush_record()?;
/// }
///
/// // Finalize with compacting (merge all blocks into one)
/// writer.finalize_with_compact(true)?;
/// ```
#[derive(Debug)]
pub struct Mf4StreamWriter<W: Write + Seek> {
    /// Writer handle
    writer: W,
    /// File path (for reference)
    path: PathBuf,
    /// Metadata
    metadata: Mf4Metadata,
    /// Configuration
    config: StreamingConfig,
    /// Data groups with streaming capability
    data_groups: Vec<StreamingDataGroup>,
    /// File state tracking
    state: WriterState,
    /// File positions for updating during finalize
    /// HD block offset
    hd_offset: Option<u64>,
    /// DG block offsets (for updating links)
    dg_offsets: Vec<u64>,
    /// CG block offsets (for updating cycle_count)
    cg_offsets: Vec<Vec<u64>>,
    /// Index of the data group with an active pending record (avoids linear scan)
    active_dg_index: Option<usize>,
}

impl Mf4StreamWriter<BufWriter<std::fs::File>> {
    /// Create a new streaming writer with default configuration
    pub fn new(path: PathBuf, metadata: Mf4Metadata) -> WriteResult<Self> {
        Self::with_config(path, metadata, StreamingConfig::default())
    }

    /// Create a new streaming writer with custom configuration
    pub fn with_config(path: PathBuf, metadata: Mf4Metadata, config: StreamingConfig) -> WriteResult<Self> {
        let file = std::fs::File::create(&path)?;
        let writer = BufWriter::new(file);

        Ok(Self {
            writer,
            path,
            metadata,
            config,
            data_groups: Vec::new(),
            state: WriterState::Initialized,
            hd_offset: None,
            dg_offsets: Vec::new(),
            cg_offsets: Vec::new(),
            active_dg_index: None,
        })
    }
}

impl<W: Write + Seek> Mf4StreamWriter<W> {
    /// Add a data group to the writer
    pub fn add_data_group(&mut self, mut dg: StreamingDataGroup) -> WriteResult<()> {
        if self.state != WriterState::Initialized {
            return Err(WriteError::AlreadyFinalized);
        }
        // Propagate configuration to data group, preserving record_size
        let record_size = dg.data_chain.record_size;
        dg.data_chain = DataBlockChain::with_record_size(self.config.clone(), record_size);
        self.data_groups.push(dg);
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &StreamingConfig {
        &self.config
    }

    /// Finalize the channel structure (ready for data)
    ///
    /// This method writes the file header blocks:
    /// - ID block (file identification)
    /// - HD block (header with metadata)
    /// - DG blocks (data groups)
    /// - CG blocks (channel groups)
    /// - CN blocks (channels)
    /// - TX blocks (channel names)
    ///
    /// After calling this method, the writer is ready to accept data records.
    pub fn finalize_structure(&mut self) -> WriteResult<()> {
        if self.state != WriterState::Initialized {
            return Err(WriteError::InvalidState {
                current: self.state.to_string(),
                required: "Initialized".to_string(),
            });
        }

        let mut block_writer = BlockWriter::new(&mut self.writer)?;

        // === Write ID Block ===
        let id_block = super::block_writer::IdBlock {
            id_file: "MDF     ".to_string(),
            id_ver: format!("{:<8}", self.metadata.version),
            id_program: "Mf4Parse".to_string(),
            id_version: self.metadata.version_num,
            id_unfin_flags: 0,
            id_custom_unfin_flags: 0,
        };
        block_writer.write_id_block(&id_block)?;

        // === Write HD Block ===
        self.hd_offset = Some(block_writer.position());
        let hd_block = super::block_writer::HdBlock {
            hd_start_time_ns: self.metadata.start_time_ns,
            hd_tz_offset: 0,
            hd_dst_offset: 0,
            hd_time_flags: 0,
            hd_time_quality: 0,
            hd_num_time_channels: 0,
            hd_dg_first: 0, // Will be updated later
            hd_fh_first: 0, // Will be updated after FH block is written
            hd_md_comment: 0,
        };
        block_writer.write_hd_block(&hd_block)?;

        // === Write FH Block (File History - mandatory for MDF 4.x) ===
        let fh_block = super::block_writer::FhBlock {
            fh_fh_next: 0,
            fh_md_comment: 0,
            fh_time_ns: self.metadata.start_time_ns,
            fh_tz_offset: 0,
            fh_dst_offset: 0,
            fh_tool_id: "Mf4Parse".to_string(),
            fh_tool_vendor: "".to_string(),
            fh_tool_version: env!("CARGO_PKG_VERSION").to_string(),
            fh_user_name: self.metadata.author.clone().unwrap_or_default(),
        };
        let fh_offset = block_writer.write_fh_block(&fh_block)?;

        // Update HD block's fh_first link
        if let Some(hd_off) = self.hd_offset {
            let hd_fh_first_offset = hd_off + 24 + 8; // After header + hd_dg_first
            block_writer.update_link(hd_fh_first_offset, fh_offset)?;
        }

        // === Write DG, CG, CN, TX blocks ===
        self.cg_offsets.clear();
        self.dg_offsets.clear();

        for (dg_idx, dg) in self.data_groups.iter_mut().enumerate() {
            // Write DG block
            let dg_offset = block_writer.position();
            self.dg_offsets.push(dg_offset);
            dg.dg_offset = Some(dg_offset);

            let dg_block = super::block_writer::DgBlock {
                dg_dg_next: 0, // Will be updated later
                dg_cg_first: 0, // Will be updated later
                dg_data: 0, // Will be updated later
                dg_md_comment: 0,
                dg_rec_id_size: dg.rec_id_size,
            };
            block_writer.write_dg_block(&dg_block)?;

            // Write CG and CN blocks for this DG
            let mut cg_offs = Vec::new();

            for (_cg_idx, cg) in dg.channel_groups.iter().enumerate() {
                // Write TX block for acquisition name
                let tx_acq_name_offset = if !cg.acq_name.is_empty() {
                    block_writer.write_tx_block(&super::block_writer::TxBlock::new(&cg.acq_name))?
                } else {
                    0
                };

                // Write SI block if acq_source is present
                let si_offset = if let Some(ref source) = cg.acq_source {
                    // Write TX block for SI name
                    let si_name_offset = block_writer.write_tx_block(
                        &super::block_writer::TxBlock::new(&source.name)
                    )?;

                    // Write TX block for SI path
                    let si_path_offset = if !source.path.is_empty() {
                        block_writer.write_tx_block(&super::block_writer::TxBlock::new(&source.path))?
                    } else {
                        0
                    };

                    // Write SI block
                    let si_flags = if source.simulated { 0x01u8 } else { 0x00u8 };
                    let si_block = super::block_writer::SiBlock {
                        si_tx_name: si_name_offset,
                        si_tx_path: si_path_offset,
                        si_md_comment: 0,
                        si_type: source.source_type as u8,
                        si_bus_type: source.bus_type as u8,
                        si_flags,
                    };
                    block_writer.write_si_block(&si_block)?
                } else {
                    0
                };

                // Write CG block
                let cg_offset = block_writer.position();
                cg_offs.push(cg_offset);

                let cg_block = super::block_writer::CgBlock {
                    cg_cg_next: 0, // Will be updated later
                    cg_cn_first: 0, // Will be updated later
                    cg_tx_acq_name: tx_acq_name_offset,
                    cg_si_acq_source: si_offset,
                    cg_md_comment: 0,
                    cg_record_id: cg.record_id,
                    cg_cycle_count: 0, // Will be updated during finalize
                    cg_data_bytes: cg.record_size,
                    cg_inval_bytes: 0,
                    cg_flags: 0,
                    cg_path_separator: 0,
                    cg_samples: 0,
                };
                block_writer.write_cg_block(&cg_block)?;

                // Write CN blocks (master first, then regular channels)
                let mut cn_offset_list = Vec::new();

                // Master channel
                if let Some(ref master) = cg.master {
                    // Write TX block for name
                    let tx_offset = block_writer.write_tx_block(
                        &super::block_writer::TxBlock::new(&master.name)
                    )?;

                    // Write CN block
                    let cn_offset = block_writer.position();
                    cn_offset_list.push(cn_offset);

                    let cn_block = super::block_writer::CnBlock {
                        cn_cn_next: 0, // Will be updated later
                        cn_composition: 0,
                        cn_tx_name: tx_offset,
                        cn_si_source: 0,
                        cn_cc_conversion: 0,
                        cn_data: 0,
                        cn_md_unit: 0,
                        cn_md_comment: 0,
                        cn_type: 2, // Master
                        cn_sync_type: master.sync_type,
                        cn_data_type: master.data_type,
                        cn_bit_offset: 0,
                        cn_byte_offset: 0,
                        cn_bit_count: master.bit_count,
                        cn_flags: 0,
                        cn_inval_bit_pos: 0,
                        cn_attachment_count: 0,
                        cn_precision: 0,
                        cn_val_limit_1: 0.0,
                        cn_val_limit_2: 0.0,
                    };
                    block_writer.write_cn_block(&cn_block)?;
                }

                // Regular channels
                let mut byte_offset: u32 = 0;
                if let Some(ref master) = cg.master {
                    byte_offset = (master.bit_count + 7) / 8;
                }

                for ch in &cg.channels {
                    // Write TX block for name
                    let tx_offset = block_writer.write_tx_block(
                        &super::block_writer::TxBlock::new(&ch.name)
                    )?;

                    // Write optional CC block (e.g. Value2Text / vtab)
                    let cc_conversion_offset = match &ch.conversion {
                        Some(ConversionParams::Value2Text { keys, texts, default }) => {
                            // Write one TX block per text entry + one default TX block
                            let mut ref_offsets: Vec<u64> = Vec::with_capacity(texts.len() + 1);
                            for text in texts {
                                let off = block_writer.write_tx_block(
                                    &super::block_writer::TxBlock::new(text)
                                )?;
                                ref_offsets.push(off);
                            }
                            let default_off = block_writer.write_tx_block(
                                &super::block_writer::TxBlock::new(default)
                            )?;
                            ref_offsets.push(default_off);

                            let cc_offset = block_writer.position();
                            let cc_val: Vec<u64> = keys.iter().map(|k| k.to_bits()).collect();
                            block_writer.write_cc_block(&super::block_writer::CcBlock {
                                cc_type: 7,
                                cc_ref_count: ref_offsets.len() as u16,
                                cc_val_count: cc_val.len() as u16,
                                cc_val,
                                cc_ref: ref_offsets,
                                ..Default::default()
                            })?;
                            cc_offset
                        }
                        Some(ConversionParams::ValueRange2Text { ranges, texts, default }) => {
                            // Write one TX block per range text + one default TX block
                            let mut ref_offsets: Vec<u64> = Vec::with_capacity(texts.len() + 1);
                            for text in texts {
                                let off = block_writer.write_tx_block(
                                    &super::block_writer::TxBlock::new(text)
                                )?;
                                ref_offsets.push(off);
                            }
                            let default_off = block_writer.write_tx_block(
                                &super::block_writer::TxBlock::new(default)
                            )?;
                            ref_offsets.push(default_off);

                            let cc_offset = block_writer.position();
                            // cc_val = [min0, max0, min1, max1, ...] as f64 bits
                            let mut cc_val: Vec<u64> = Vec::with_capacity(ranges.len() * 2);
                            for (min, max) in ranges {
                                cc_val.push(min.to_bits());
                                cc_val.push(max.to_bits());
                            }
                            block_writer.write_cc_block(&super::block_writer::CcBlock {
                                cc_type: 8,
                                cc_ref_count: ref_offsets.len() as u16,
                                cc_val_count: cc_val.len() as u16,
                                cc_val,
                                cc_ref: ref_offsets,
                                ..Default::default()
                            })?;
                            cc_offset
                        }
                        _ => 0,
                    };

                    // Write CN block
                    let cn_offset = block_writer.position();
                    cn_offset_list.push(cn_offset);

                    let cn_block = super::block_writer::CnBlock {
                        cn_cn_next: 0, // Will be updated later
                        cn_composition: 0,
                        cn_tx_name: tx_offset,
                        cn_si_source: 0,
                        cn_cc_conversion: cc_conversion_offset,
                        cn_data: 0,
                        cn_md_unit: 0,
                        cn_md_comment: 0,
                        cn_type: 0, // Fixed length
                        cn_sync_type: 0,
                        cn_data_type: ch.data_type,
                        cn_bit_offset: 0,
                        cn_byte_offset: byte_offset,
                        cn_bit_count: ch.bit_count,
                        cn_flags: 0,
                        cn_inval_bit_pos: 0,
                        cn_attachment_count: 0,
                        cn_precision: 0,
                        cn_val_limit_1: 0.0,
                        cn_val_limit_2: 0.0,
                    };
                    block_writer.write_cn_block(&cn_block)?;

                    byte_offset += (ch.bit_count + 7) / 8;
                }

                // Store CN offsets for this CG
                // Update CN next links
                for i in 0..cn_offset_list.len() - 1 {
                    let next_offset = cn_offset_list[i + 1];
                    let link_offset = cn_offset_list[i] + 24; // Offset to cn_cn_next
                    block_writer.update_link(link_offset, next_offset)?;
                }

                // Update CG cn_first link
                if !cn_offset_list.is_empty() {
                    let cg_cn_first_offset = cg_offset + 24 + 8; // After header + cg_cg_next
                    block_writer.update_link(cg_cn_first_offset, cn_offset_list[0])?;
                }
            }

            self.cg_offsets.push(cg_offs);

            // Store data area offset for this DG
            dg.data_area_offset = Some(block_writer.position());

            // Update CG next links
            for i in 0..self.cg_offsets[dg_idx].len() - 1 {
                let next_offset = self.cg_offsets[dg_idx][i + 1];
                let cg_cg_next_offset = self.cg_offsets[dg_idx][i] + 24; // Offset to cg_cg_next
                block_writer.update_link(cg_cg_next_offset, next_offset)?;
            }

            // Update DG cg_first link
            if !self.cg_offsets[dg_idx].is_empty() {
                let dg_cg_first_offset = dg_offset + 24 + 8; // After header + dg_dg_next
                block_writer.update_link(dg_cg_first_offset, self.cg_offsets[dg_idx][0])?;
            }
        }

        // Update DG next links
        for i in 0..self.dg_offsets.len() - 1 {
            let next_offset = self.dg_offsets[i + 1];
            let dg_dg_next_offset = self.dg_offsets[i] + 24; // Offset to dg_dg_next
            block_writer.update_link(dg_dg_next_offset, next_offset)?;
        }

        // Update HD dg_first link
        if let Some(hd_off) = self.hd_offset {
            if !self.dg_offsets.is_empty() {
                let hd_dg_first_offset = hd_off + 24; // Offset to hd_dg_first
                block_writer.update_link(hd_dg_first_offset, self.dg_offsets[0])?;
            }
        }

        self.state = WriterState::StructureReady;
        Ok(())
    }

    /// Start a new record for the given data group and channel group
    pub fn start_record(&mut self, dg_index: usize, cg_index: usize) -> WriteResult<()> {
        if self.state != WriterState::StructureReady && self.state != WriterState::Writing {
            return Err(WriteError::InvalidState {
                current: self.state.to_string(),
                required: "StructureReady or Writing".to_string(),
            });
        }

        let dg = self.data_groups.get_mut(dg_index)
            .ok_or(WriteError::ChannelNotFound { name: format!("DataGroup[{}]", dg_index) })?;

        dg.start_record(cg_index)?;
        self.active_dg_index = Some(dg_index);
        self.state = WriterState::Writing;
        Ok(())
    }

    /// Set a channel value in the current pending record
    pub fn set_channel_value<T: RecordValue>(&mut self, channel_name: &str, value: T) -> WriteResult<()> {
        let dg_idx = self.active_dg_index.ok_or(WriteError::InvalidState {
            current: "No pending record".to_string(),
            required: "Record started".to_string(),
        })?;
        self.data_groups[dg_idx].set_channel_value(channel_name, value)
    }

    /// Complete and flush the current record
    pub fn flush_record(&mut self) -> WriteResult<()> {
        let dg_idx = self.active_dg_index.take().ok_or(WriteError::InvalidState {
            current: "No pending record".to_string(),
            required: "Record started".to_string(),
        })?;

        let dg = &mut self.data_groups[dg_idx];
        dg.flush_record()?;

        if dg.is_block_full() {
            self.flush_data_block()?;
        }

        Ok(())
    }

    /// Write a complete record with all channel values in one call
    ///
    /// Values must be in channel definition order: master (time) first, then
    /// data channels in the order they were added. This is a shorthand for
    /// the single-DG, single-CG case (dg_index=0, cg_index=0).
    ///
    /// # Example
    /// ```ignore
    /// // With channels: time, voltage, current
    /// writer.write_record(&[0.001, 3.14, 1.5])?;
    /// ```
    pub fn write_record(&mut self, values: &[f64]) -> WriteResult<()> {
        let dg = self.data_groups.get(0)
            .ok_or(WriteError::ChannelNotFound { name: "No data groups".to_string() })?;
        let cg = dg.channel_groups.get(0)
            .ok_or(WriteError::ChannelNotFound { name: "No channel groups".to_string() })?;

        let expected = cg.channels.len() + if cg.master.is_some() { 1 } else { 0 };
        if values.len() != expected {
            return Err(WriteError::InvalidChannelConfig(
                format!("Expected {} values but got {} (channels: master={}, data={})",
                    expected, values.len(),
                    if cg.master.is_some() { 1 } else { 0 },
                    cg.channels.len())
            ));
        }

        // Collect channel names to avoid borrow conflicts
        let mut channel_names: Vec<String> = Vec::with_capacity(expected);
        if let Some(ref m) = cg.master {
            channel_names.push(m.name.clone());
        }
        for ch in &cg.channels {
            channel_names.push(ch.name.clone());
        }

        self.start_record(0, 0)?;
        for (i, &val) in values.iter().enumerate() {
            self.set_channel_value(&channel_names[i], val)?;
        }
        self.flush_record()
    }

    /// Flush the current data block to disk
    ///
    /// This method writes accumulated data in the buffer as a DT block
    /// and tracks the offset for the DL chain.
    fn flush_data_block(&mut self) -> WriteResult<()> {
        // Find the data group with pending data
        for dg in &mut self.data_groups {
            if dg.data_chain.buffer_size() > 0 && dg.data_chain.is_buffer_full() {
                // Create block writer at current position
                let mut block_writer = BlockWriter::new(&mut self.writer)?;

                // Write DT block
                let dt_offset = dg.data_chain.finalize_current_block(&mut block_writer)?;

                if let Some(_offset) = dt_offset {
                    // Track this block for DL chain
                    // The DL block will be written during finalize
                }
            }
        }
        Ok(())
    }

    /// Flush buffered data to disk
    pub fn flush(&mut self) -> WriteResult<()> {
        self.writer.flush()?;
        Ok(())
    }

    /// Finalize the file without compacting
    pub fn finalize(&mut self) -> WriteResult<()> {
        self.finalize_with_compact(false)
    }

    /// Finalize the file with optional compacting
    ///
    /// # Arguments
    /// * `compact` - If true, write all data as a single DT block (uncompressed only).
    ///               If false, split data into record-aligned chunks linked by DL.
    ///
    /// # Errors
    /// Returns `WriteError::InvalidChannelConfig` if `compact = true` and compression
    /// is enabled — these modes are mutually exclusive. Use `finalize_with_compact(false)`
    /// (stream mode) for compressed output.
    ///
    /// # Details
    /// When `compact` is true (uncompressed only):
    /// - A single DT block is written: DG.dg_data → DT
    ///
    /// When `compact` is false (stream write mode):
    /// - Data is split into record-aligned chunks
    /// - Each chunk becomes a DT block (or DZ block if compressed)
    /// - A DL block links all data blocks
    /// - If compressed: DG.dg_data → HL → DL → [DZ₁, DZ₂, ...]
    /// - If uncompressed: DG.dg_data → DL → [DT₁, DT₂, ...]
    /// - Each DZ block's uncompressed size ≤ 4MB
    /// - No record spans two data blocks
    pub fn finalize_with_compact(&mut self, compact: bool) -> WriteResult<()> {
        if compact && self.config.enable_compression {
            return Err(WriteError::InvalidChannelConfig(
                "compact mode and compression are mutually exclusive; \
                 use finalize_with_compact(false) for compressed output".to_string(),
            ));
        }
        if self.state == WriterState::Finalized {
            return Err(WriteError::AlreadyFinalized);
        }

        // 1. Flush any pending records
        for dg in &mut self.data_groups {
            if dg.pending_record.is_some() {
                dg.flush_record()?;
            }
        }

        // 2. Write data blocks and update links
        let mut block_writer = BlockWriter::new(&mut self.writer)?;

        for (dg_idx, dg) in self.data_groups.iter_mut().enumerate() {
            // Take the shared buffer and sort by master-channel time for multi-CG DGs.
            // The MDF4 specification requires all records within a DG to be sorted by
            // the master channel time. Sorting here ensures compatibility with all
            // conformant MDF4 readers regardless of the order records were written.
            let raw = std::mem::take(&mut dg.shared_buffer);
            let data = if dg.channel_groups.len() > 1 {
                sort_records_by_time(raw, dg)
            } else {
                raw
            };

            let cycle_count = dg.cycle_counts.iter().sum();
            let data_len = data.len() as u64;

            if compact {
                // Compact mode: write a single DT or DZ block
                let should_compress = self.config.enable_compression
                    && data_len >= self.config.compression_threshold;

                let block_offset = if should_compress {
                    if data_len > MAX_DZ_UNCOMPRESSED_SIZE {
                        // MDF4 forbids a single DZ block with original_length > 4MB.
                        // Write a DL-chained series of DZ blocks instead.
                        dg.data_chain.write_chunked_chain(&mut block_writer, data)?
                    } else {
                        let compressor = super::compression::Compressor {
                            compression_type: super::compression::CompressionType::Deflate,
                            level: self.config.compression_level,
                            column_count: None,
                        };
                        let (compressed_data, original_len) = compressor.compress(&data)?;
                        let dz = super::block_writer::DzBlock {
                            dz_org_data_length: original_len,
                            dz_data_length: compressed_data.len() as u64,
                            dz_zip_type: 0,
                            dz_zip_parameter: 0,
                            data: compressed_data,
                        };
                        block_writer.write_dz_block(&dz)?
                    }
                } else {
                    block_writer.write_dt_block(&super::block_writer::DtBlock::new(data))?
                };

                // Update DG data link → single block
                if let Some(dg_off) = dg.dg_offset {
                    let dg_data_offset = dg_off + 24 + 16; // After header + dg_dg_next + dg_cg_first
                    block_writer.update_link(dg_data_offset, block_offset)?;
                }
            } else {
                // Non-compact (stream write) mode:
                // Split data into record-aligned chunks and write as DL chain.
                // DataBlockChain handles chunk splitting, DZ/DT writing, DL + HL creation.
                let top_offset = dg.data_chain.write_chunked_chain(
                    &mut block_writer,
                    data,
                )?;

                // Update DG data link → top-level block (DL or HL)
                if let Some(dg_off) = dg.dg_offset {
                    let dg_data_offset = dg_off + 24 + 16;
                    block_writer.update_link(dg_data_offset, top_offset)?;
                }
            }

            // 3. Update cycle counts in CG blocks
            for (cg_idx, _cg) in dg.channel_groups.iter().enumerate() {
                if let Some(&cg_offset) = self.cg_offsets.get(dg_idx).and_then(|v| v.get(cg_idx)) {
                    // CG cycle_count data field is at offset 80 from CG block start
                    // (24 header + 48 links + 8 record_id = 80)
                    let cycle_count_offset = cg_offset + 80;
                    let cycle_count_bytes = if dg.channel_groups.len() == 1 {
                        cycle_count
                    } else {
                        dg.cycle_counts[cg_idx]
                    };

                    // Seek and update cycle_count (uses update_link to preserve current position)
                    block_writer.update_link(cycle_count_offset, cycle_count_bytes)?;
                }
            }
        }

        // 4. Update DG next links
        for i in 0..self.dg_offsets.len().saturating_sub(1) {
            let next_offset = self.dg_offsets[i + 1];
            let dg_dg_next_offset = self.dg_offsets[i] + 24;
            block_writer.update_link(dg_dg_next_offset, next_offset)?;
        }

        self.flush()?;
        self.state = WriterState::Finalized;
        Ok(())
    }

    /// Get the current state
    pub fn state(&self) -> WriterState {
        self.state
    }

    /// Get the metadata
    pub fn metadata(&self) -> &Mf4Metadata {
        &self.metadata
    }

    /// Get the file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get total records written across all data groups
    pub fn total_records(&self) -> u64 {
        self.data_groups.iter().map(|dg| dg.total_cycle_count()).sum()
    }
}
