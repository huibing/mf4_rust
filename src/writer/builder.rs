//! Mf4Builder - High-level API for one-time MF4 file creation
//!
//! This module provides a builder pattern for creating MF4 files in a single write operation.

use std::io::{Seek, Write};
use std::path::PathBuf;

use super::error::{WriteError, WriteResult};

// ============================================================================
// Metadata Structures
// ============================================================================

/// Metadata for MF4 file header
#[derive(Debug, Clone)]
pub struct Mf4Metadata {
    /// File version string (e.g., "4.10", "4.11")
    pub version: String,
    /// Version number (e.g., 410, 411)
    pub version_num: u16,
    /// Start timestamp in nanoseconds since 1970-01-01
    pub start_time_ns: u64,
    /// Author information
    pub author: Option<String>,
    /// Organization
    pub organization: Option<String>,
    /// Project name
    pub project: Option<String>,
    /// Subject/comment
    pub comment: Option<String>,
}

impl Default for Mf4Metadata {
    fn default() -> Self {
        Self {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0),
            author: None,
            organization: None,
            project: None,
            comment: None,
        }
    }
}

impl Mf4Metadata {
    /// Create new metadata with current timestamp
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the version
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self.version_num = match version {
            "4.00" => 400,
            "4.10" => 410,
            "4.11" => 411,
            "4.20" => 420,
            _ => 410,
        };
        self
    }

    /// Set the start time
    pub fn with_start_time(mut self, time_ns: u64) -> Self {
        self.start_time_ns = time_ns;
        self
    }

    /// Set the author
    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    /// Set the organization
    pub fn with_organization(mut self, org: &str) -> Self {
        self.organization = Some(org.to_string());
        self
    }

    /// Set the project name
    pub fn with_project(mut self, project: &str) -> Self {
        self.project = Some(project.to_string());
        self
    }

    /// Set the comment
    pub fn with_comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }
}

// ============================================================================
// Compression Configuration
// ============================================================================

/// Compression configuration for data blocks
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Compression type: 0 = Deflate, 1 = Transpose + Deflate
    pub zip_type: u8,
    /// Minimum data size to trigger compression (bytes)
    pub min_size: u64,
    /// Compression level (1-9, default 6)
    pub level: u8,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            zip_type: 0,
            min_size: 1_000_000, // 1 MB
            level: 6,
        }
    }
}

impl CompressionConfig {
    /// Create new compression config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set compression type (0=Deflate, 1=Transpose+Deflate)
    pub fn with_zip_type(mut self, zip_type: u8) -> Self {
        self.zip_type = zip_type;
        self
    }

    /// Set minimum size threshold for compression
    pub fn with_min_size(mut self, min_size: u64) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set compression level (1-9)
    pub fn with_level(mut self, level: u8) -> Self {
        self.level = level.clamp(1, 9);
        self
    }
}

// ============================================================================
// Conversion Builder
// ============================================================================

/// Conversion parameters for channel data transformation
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum ConversionParams {
    /// No conversion (1:1)
    #[default]
    OneToOne,
    /// Linear conversion: y = p1 + p2 * x
    Linear { p1: f64, p2: f64 },
    /// Rational conversion: y = (p0*x² + p1*x + p2) / (p3*x² + p4*x + p5)
    Rational { coeffs: [f64; 6] },
    /// Table lookup with optional interpolation
    Table {
        keys: Vec<f64>,
        values: Vec<f64>,
        interpolate: bool,
    },
    /// Value to text conversion
    Value2Text {
        keys: Vec<f64>,
        texts: Vec<String>,
        default: String,
    },
    /// Value range to text conversion
    ValueRange2Text {
        ranges: Vec<(f64, f64)>, // (min, max) pairs
        texts: Vec<String>,
        default: String,
    },
}


/// Builder for conversion rules
#[derive(Debug, Clone)]
pub struct ConversionBuilder {
    /// Conversion type (0=1:1, 1=Linear, etc.)
    pub cc_type: u8,
    /// Conversion parameters
    pub params: ConversionParams,
    /// Unit string
    pub unit: Option<String>,
    /// Comment/description
    pub comment: Option<String>,
}

impl Default for ConversionBuilder {
    fn default() -> Self {
        Self {
            cc_type: 0,
            params: ConversionParams::OneToOne,
            unit: None,
            comment: None,
        }
    }
}

impl ConversionBuilder {
    /// Create a new conversion builder (default: 1:1)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a linear conversion: y = p1 + p2 * x
    pub fn linear(p1: f64, p2: f64) -> Self {
        Self {
            cc_type: 1,
            params: ConversionParams::Linear { p1, p2 },
            unit: None,
            comment: None,
        }
    }

    /// Create a rational conversion
    pub fn rational(coeffs: [f64; 6]) -> Self {
        Self {
            cc_type: 2,
            params: ConversionParams::Rational { coeffs },
            unit: None,
            comment: None,
        }
    }

    /// Create a table lookup conversion with interpolation
    pub fn table_interpolate(keys: Vec<f64>, values: Vec<f64>) -> Self {
        Self {
            cc_type: 4,
            params: ConversionParams::Table {
                keys,
                values,
                interpolate: true,
            },
            unit: None,
            comment: None,
        }
    }

    /// Create a table lookup conversion without interpolation
    pub fn table(keys: Vec<f64>, values: Vec<f64>) -> Self {
        Self {
            cc_type: 5,
            params: ConversionParams::Table {
                keys,
                values,
                interpolate: false,
            },
            unit: None,
            comment: None,
        }
    }

    /// Set the unit
    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    /// Set the comment
    pub fn with_comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }
}

// ============================================================================
// Channel Builder
// ============================================================================

/// Sync type for channel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum SyncType {
    /// No synchronization
    #[default]
    None = 0,
    /// Time synchronization
    Time = 1,
    /// Angle synchronization
    Angle = 2,
    /// Distance synchronization
    Distance = 3,
    /// Index synchronization
    Index = 4,
}


/// Channel type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ChannelType {
    /// Fixed length data channel
    #[default]
    FixedLength = 0,
    /// Variable length data channel (VLSD)
    VariableLength = 1,
    /// Master channel (time/angle/distance)
    Master = 2,
    /// Virtual master channel
    VirtualMaster = 3,
}


/// Builder for a channel definition
#[derive(Debug, Clone)]
pub struct ChannelBuilder {
    /// Channel name
    pub name: String,
    /// Data type (0=UINT_LE, 1=UINT_BE, 2=INT_LE, 3=INT_BE, 4=FLOAT_LE, 5=FLOAT_BE, 6=STRING_LE, etc.)
    pub data_type: u8,
    /// Number of bits (8, 16, 32, 64 for numeric types)
    pub bit_count: u32,
    /// Unit string
    pub unit: Option<String>,
    /// Comment/description
    pub comment: Option<String>,
    /// Sync type
    pub sync_type: SyncType,
    /// Channel type
    pub cn_type: ChannelType,
    /// Conversion configuration
    pub conversion: Option<ConversionBuilder>,
    /// Array dimensions (for array channels)
    pub array_dims: Option<Vec<u32>>,
    /// Source information
    pub source_name: Option<String>,
    pub source_path: Option<String>,
}

impl ChannelBuilder {
    /// Create a new channel builder
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: 0,
            bit_count: 64,
            unit: None,
            comment: None,
            sync_type: SyncType::None,
            cn_type: ChannelType::FixedLength,
            conversion: None,
            array_dims: None,
            source_name: None,
            source_path: None,
        }
    }

    /// Create a new master time channel
    pub fn new_master_time(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: 5, // FLOAT64 LE
            bit_count: 64,
            unit: Some("s".to_string()),
            comment: None,
            sync_type: SyncType::Time,
            cn_type: ChannelType::Master,
            conversion: None,
            array_dims: None,
            source_name: None,
            source_path: None,
        }
    }

    /// Set the data type
    /// - 0: UINT_LE, 1: UINT_BE
    /// - 2: INT_LE, 3: INT_BE
    /// - 4: FLOAT_LE (IEEE 754), 5: FLOAT_BE
    /// - 6: STRING_LE (UTF-8), 7: STRING_BE (UTF-8)
    /// - 8: STRING_UTF16_LE, 9: STRING_UTF16_BE
    /// - 10: BYTE_ARRAY
    pub fn data_type(mut self, data_type: u8) -> Self {
        self.data_type = data_type;
        self
    }

    /// Set the bit count (8, 16, 32, 64)
    pub fn bit_count(mut self, bit_count: u32) -> Self {
        self.bit_count = bit_count;
        self
    }

    /// Set the unit
    pub fn unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    /// Set the comment
    pub fn comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }

    /// Set the sync type
    pub fn sync_type(mut self, sync_type: SyncType) -> Self {
        self.sync_type = sync_type;
        self
    }

    /// Set the channel type
    pub fn cn_type(mut self, cn_type: ChannelType) -> Self {
        self.cn_type = cn_type;
        self
    }

    /// Set the conversion
    pub fn conversion(mut self, conversion: ConversionBuilder) -> Self {
        self.conversion = Some(conversion);
        self
    }

    /// Set array dimensions (for array channels)
    pub fn array_dims(mut self, dims: Vec<u32>) -> Self {
        self.array_dims = Some(dims);
        self
    }

    /// Set source information
    pub fn source(mut self, name: &str, path: &str) -> Self {
        self.source_name = Some(name.to_string());
        self.source_path = Some(path.to_string());
        self
    }

    /// Build the channel (validates configuration)
    pub fn build(self) -> WriteResult<Self> {
        // Validate data type
        if self.data_type > 10 {
            return Err(WriteError::InvalidChannelConfig(format!(
                "Invalid data type {} for channel '{}'",
                self.data_type, self.name
            )));
        }

        // Validate bit count for numeric types
        if self.data_type <= 5 {
            let valid_bits = [8, 16, 32, 64];
            if !valid_bits.contains(&self.bit_count) {
                return Err(WriteError::InvalidChannelConfig(format!(
                    "Invalid bit count {} for numeric channel '{}'",
                    self.bit_count, self.name
                )));
            }
        }

        Ok(self)
    }
}

// ============================================================================
// Channel Group Builder
// ============================================================================

/// Builder for a channel group definition
#[derive(Debug, Clone)]
pub struct ChannelGroupBuilder {
    /// Acquisition name
    pub acq_name: String,
    /// Record ID (for multi-CG scenarios)
    pub record_id: u64,
    /// Channels in this group (excluding master)
    pub channels: Vec<ChannelBuilder>,
    /// Master channel (time/angle/distance)
    pub master: Option<ChannelBuilder>,
    /// Channel group flags
    pub flags: u16,
    /// Comment
    pub comment: Option<String>,
}

impl ChannelGroupBuilder {
    /// Create a new channel group builder
    pub fn new() -> Self {
        Self {
            acq_name: String::new(),
            record_id: 0,
            channels: Vec::new(),
            master: None,
            flags: 0,
            comment: None,
        }
    }

    /// Set the acquisition name
    pub fn name(mut self, name: &str) -> Self {
        self.acq_name = name.to_string();
        self
    }

    /// Set the record ID
    pub fn record_id(mut self, record_id: u64) -> Self {
        self.record_id = record_id;
        self
    }

    /// Set the master channel
    pub fn master(mut self, channel: ChannelBuilder) -> Self {
        self.master = Some(channel);
        self
    }

    /// Add a channel to the group
    pub fn channel(mut self, channel: ChannelBuilder) -> Self {
        self.channels.push(channel);
        self
    }

    /// Set the comment
    pub fn comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }

    /// Build the channel group (validates configuration)
    pub fn build(self) -> WriteResult<Self> {
        if self.channels.is_empty() && self.master.is_none() {
            return Err(WriteError::InvalidChannelConfig(
                "Channel group must have at least one channel".to_string(),
            ));
        }
        Ok(self)
    }
}

impl Default for ChannelGroupBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Data Group Builder
// ============================================================================

/// Builder for a data group definition
#[derive(Debug, Clone)]
pub struct DataGroupBuilder {
    /// Channel groups within this data group
    pub channel_groups: Vec<ChannelGroupBuilder>,
    /// Record ID size (0, 1, 2, 4, 8 bytes)
    pub rec_id_size: u8,
    /// Comment
    pub comment: Option<String>,
}

impl DataGroupBuilder {
    /// Create a new data group builder
    pub fn new() -> Self {
        Self {
            channel_groups: Vec::new(),
            rec_id_size: 0,
            comment: None,
        }
    }

    /// Add a channel group
    pub fn channel_group(mut self, cg: ChannelGroupBuilder) -> Self {
        self.channel_groups.push(cg);
        // Update rec_id_size based on number of channel groups
        if self.channel_groups.len() == 1 {
            self.rec_id_size = 0; // No record ID needed for single CG
        } else if self.channel_groups.len() <= 255 {
            self.rec_id_size = 1;
        } else if self.channel_groups.len() <= 65535 {
            self.rec_id_size = 2;
        } else {
            self.rec_id_size = 4;
        }
        self
    }

    /// Set the comment
    pub fn comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }

    /// Build the data group (validates configuration and auto-assigns record IDs)
    pub fn build(mut self) -> WriteResult<Self> {
        if self.channel_groups.is_empty() {
            return Err(WriteError::MissingField("channel_groups".to_string()));
        }
        let num_cgs = self.channel_groups.len();
        // Auto-assign record IDs if not set (1-indexed for multiple CGs)
        for (idx, cg) in self.channel_groups.iter_mut().enumerate() {
            if cg.record_id == 0 && num_cgs > 1 {
                cg.record_id = (idx + 1) as u64;
            }
        }
        Ok(self)
    }
}

impl Default for DataGroupBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Mf4Builder - Main Builder for One-time Write
// ============================================================================

/// Builder for creating MF4 files in a single write operation
///
/// # Example
///
/// ```ignore
/// use mf4_parse::writer::{Mf4Builder, Mf4Metadata, ChannelBuilder, ChannelGroupBuilder, DataGroupBuilder};
///
/// let mut builder = Mf4Builder::new(Mf4Metadata::default());
///
/// // Define channels
/// let time_ch = ChannelBuilder::new_master_time("time");
/// let temp_ch = ChannelBuilder::new("Temperature")
///     .data_type(5)  // FLOAT64
///     .unit("°C")
///     .build()?;
///
/// // Create channel group
/// let cg = ChannelGroupBuilder::new()
///     .name("Measurement")
///     .master(time_ch)
///     .channel(temp_ch)
///     .build()?;
///
/// // Create data group
/// let dg = DataGroupBuilder::new()
///     .channel_group(cg)
///     .build();
///
/// builder.add_data_group(dg);
///
/// // Set data
/// builder.set_channel_data("time", &vec![0.0, 0.1, 0.2])?;
/// builder.set_channel_data("Temperature", &vec![20.0, 21.0, 22.0])?;
///
/// // Write file
/// builder.write("output.mf4".into())?;
/// ```
#[derive(Debug)]
pub struct Mf4Builder {
    /// File metadata
    metadata: Mf4Metadata,
    /// Collection of data groups to write
    data_groups: Vec<DataGroupBuilder>,
    /// Compression settings
    compression: Option<CompressionConfig>,
    /// Channel data storage (channel_name -> data)
    channel_data: std::collections::HashMap<String, Vec<u8>>,
    /// Channel data types (for validation)
    channel_types: std::collections::HashMap<String, (u8, u32)>, // (data_type, bit_count)
}

impl Mf4Builder {
    /// Create a new Mf4Builder with the given metadata
    pub fn new(metadata: Mf4Metadata) -> Self {
        Self {
            metadata,
            data_groups: Vec::new(),
            compression: None,
            channel_data: std::collections::HashMap::new(),
            channel_types: std::collections::HashMap::new(),
        }
    }

    /// Create a new Mf4Builder with default metadata
    pub fn with_defaults() -> Self {
        Self::new(Mf4Metadata::default())
    }

    /// Set compression configuration
    pub fn set_compression(&mut self, config: CompressionConfig) {
        self.compression = Some(config);
    }

    /// Add a data group
    pub fn add_data_group(&mut self, dg: DataGroupBuilder) {
        // Register channel types
        for cg in &dg.channel_groups {
            for ch in &cg.channels {
                self.channel_types.insert(
                    ch.name.clone(),
                    (ch.data_type, ch.bit_count),
                );
            }
            if let Some(ref master) = cg.master {
                self.channel_types.insert(
                    master.name.clone(),
                    (master.data_type, master.bit_count),
                );
            }
        }
        self.data_groups.push(dg);
    }

    /// Set channel data (generic version using serialization)
    ///
    /// This method serializes the data into raw bytes for storage.
    pub fn set_channel_data<T: ChannelData>(&mut self, channel_name: &str, data: &[T]) -> WriteResult<()> {
        let (data_type, bit_count) = self.channel_types
            .get(channel_name)
            .ok_or_else(|| WriteError::ChannelNotFound { name: channel_name.to_string() })?;

        let raw_data = T::serialize_to_bytes(data, *data_type, *bit_count)?;
        self.channel_data.insert(channel_name.to_string(), raw_data);
        Ok(())
    }

    /// Write the MF4 file to the specified path
    pub fn write(&self, path: PathBuf) -> WriteResult<()> {
        use std::fs::File;
        use std::io::{BufWriter, Seek, SeekFrom, Write};

        // Create file
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        // Seek past ID block (fixed at 0x00, 104 bytes) and HD block (fixed at 0x40)
        // Actually, ID block ends at 0x68 (104 bytes), HD block starts there
        // Let's write ID block first

        // === Write ID Block ===
        writer.seek(SeekFrom::Start(0))?;
        let id_block = super::block_writer::IdBlock {
            id_file: "MDF     ".to_string(),
            id_ver: format!("{:<8}", self.metadata.version),
            id_program: "Mf4Parse".to_string(),
            id_version: self.metadata.version_num,
            id_unfin_flags: 0,
            id_custom_unfin_flags: 0,
        };
        self.write_id_block_raw(&mut writer, &id_block)?;

        // === Write HD Block ===
        // HD block starts right after ID block
        let hd_offset = writer.stream_position()?;
        let mut hd_block = super::block_writer::HdBlock {
            hd_start_time_ns: self.metadata.start_time_ns,
            hd_tz_offset: 0,
            hd_dst_offset: 0,
            hd_time_flags: 0,
            hd_time_quality: 0,
            hd_num_time_channels: 0,
            hd_dg_first: 0,
            hd_fh_first: 0,
            hd_md_comment: 0,
        };

        // Write HD block placeholder (will update dg_first later)
        self.write_hd_block_raw(&mut writer, &hd_block)?;

        // === Calculate all offsets ===
        // We need to know the total size to compute data block positions
        let mut current_offset = writer.stream_position()?;

        // Track offsets for each block
        let mut dg_offsets: Vec<u64> = Vec::new();
        let mut cg_offsets: Vec<Vec<u64>> = Vec::new();
        let mut cn_offsets: Vec<Vec<Vec<u64>>> = Vec::new();
        let mut tx_offsets: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let mut data_offsets: Vec<u64> = Vec::new();

        // First pass: calculate sizes and reserve space
        for dg in self.data_groups.iter() {
            // Align and record DG offset
            current_offset = (current_offset + 7) & !7;
            dg_offsets.push(current_offset);
            current_offset += 64; // DG block size

            let mut cg_offs = Vec::new();
            let mut cn_offs = Vec::new();

            for cg in dg.channel_groups.iter() {
                // Align and record CG offset
                current_offset = (current_offset + 7) & !7;
                cg_offs.push(current_offset);
                current_offset += 104; // CG block size (104 bytes)

                let mut cn_list = Vec::new();

                // Master channel
                if let Some(ref master) = cg.master {
                    // TX block for name
                    current_offset = (current_offset + 7) & !7;
                    let tx_offset = current_offset;
                    tx_offsets.insert(master.name.clone(), tx_offset);
                    current_offset += 24 + ((master.name.len() + 1 + 7) & !7) as u64;

                    // CN block
                    current_offset = (current_offset + 7) & !7;
                    cn_list.push(current_offset);
                    current_offset += 160; // CN block size
                }

                // Regular channels
                for ch in &cg.channels {
                    // TX block for name
                    current_offset = (current_offset + 7) & !7;
                    let tx_offset = current_offset;
                    tx_offsets.insert(ch.name.clone(), tx_offset);
                    current_offset += 24 + ((ch.name.len() + 1 + 7) & !7) as u64;

                    // CN block
                    current_offset = (current_offset + 7) & !7;
                    cn_list.push(current_offset);
                    current_offset += 160;
                }

                cn_offs.push(cn_list);
            }

            cg_offsets.push(cg_offs);
            cn_offsets.push(cn_offs);

            // Data block
            current_offset = (current_offset + 7) & !7;
            data_offsets.push(current_offset);
        }

        // === Second pass: write blocks with correct links ===
        writer.seek(SeekFrom::Start(hd_offset))?;

        // Update HD block with dg_first
        hd_block.hd_dg_first = if !dg_offsets.is_empty() { dg_offsets[0] } else { 0 };
        self.write_hd_block_raw(&mut writer, &hd_block)?;

        // Write DG, CG, CN, TX blocks
        for (dg_idx, dg) in self.data_groups.iter().enumerate() {
            // Write DG block
            writer.seek(SeekFrom::Start(dg_offsets[dg_idx]))?;
            let dg_next = if dg_idx + 1 < dg_offsets.len() { dg_offsets[dg_idx + 1] } else { 0 };
            let cg_first = if !cg_offsets[dg_idx].is_empty() { cg_offsets[dg_idx][0] } else { 0 };
            let data_offset = data_offsets[dg_idx];

            self.write_dg_block_raw(&mut writer, dg_next, cg_first, data_offset, 0, dg.rec_id_size)?;

            // Write CG and CN blocks
            for (cg_idx, cg) in dg.channel_groups.iter().enumerate() {
                writer.seek(SeekFrom::Start(cg_offsets[dg_idx][cg_idx]))?;

                // Calculate record info
                let mut record_size: u32 = 0;

                if let Some(ref master) = cg.master {
                    record_size += master.bit_count.div_ceil(8);
                }
                for ch in &cg.channels {
                    record_size += ch.bit_count.div_ceil(8);
                }

                // Get cycle count from data
                // Each channel stores its own data independently, so we get the element count
                // from the master channel (or first channel) based on its byte size
                let cycle_count = if let Some(ref master) = cg.master {
                    let master_byte_size = master.bit_count.div_ceil(8) as u64;
                    self.channel_data.get(&master.name)
                        .map(|d| d.len() as u64 / master_byte_size)
                        .unwrap_or(0)
                } else if !cg.channels.is_empty() {
                    let first_ch = &cg.channels[0];
                    let first_byte_size = first_ch.bit_count.div_ceil(8) as u64;
                    self.channel_data.get(&first_ch.name)
                        .map(|d| d.len() as u64 / first_byte_size)
                        .unwrap_or(0)
                } else {
                    0
                };

                let cn_first = if !cn_offsets[dg_idx][cg_idx].is_empty() { cn_offsets[dg_idx][cg_idx][0] } else { 0 };

                self.write_cg_block_raw(&mut writer,
                    if cg_idx + 1 < cg_offsets[dg_idx].len() { cg_offsets[dg_idx][cg_idx + 1] } else { 0 },
                    cn_first,
                    0, // tx_acq_name
                    0, // si_acq_source
                    0, // md_comment
                    cg.record_id,
                    cycle_count,
                    record_size,
                    0, // invalid_bytes
                )?;

                // Write TX blocks and CN blocks
                let mut cn_idx = 0;
                if let Some(ref master) = cg.master {
                    // Write TX block for master channel name
                    if let Some(&tx_offset) = tx_offsets.get(&master.name) {
                        writer.seek(SeekFrom::Start(tx_offset))?;
                        self.write_tx_block_raw(&mut writer, &master.name)?;
                    }

                    // Write CN block
                    writer.seek(SeekFrom::Start(cn_offsets[dg_idx][cg_idx][cn_idx]))?;
                    let cn_next = if cn_idx + 1 < cn_offsets[dg_idx][cg_idx].len() { cn_offsets[dg_idx][cg_idx][cn_idx + 1] } else { 0 };

                    self.write_cn_block_raw(&mut writer,
                        cn_next,
                        0, // composition
                        *tx_offsets.get(&master.name).unwrap_or(&0),
                        0, // si_source
                        0, // cc_conversion
                        0, // cn_data
                        0, // md_unit
                        0, // md_comment
                        2, // cn_type = master
                        1, // sync_type = time
                        master.data_type,
                        0, // bit_offset
                        0, // byte_offset
                        master.bit_count,
                    )?;
                    cn_idx += 1;
                }

                // Calculate cumulative byte offsets for channels
                let mut cumulative_byte_offset: u32 = cg.master.as_ref().map(|m| m.bit_count.div_ceil(8) as u32).unwrap_or(0);

                for ch in &cg.channels {
                    // Write TX block for channel name
                    if let Some(&tx_offset) = tx_offsets.get(&ch.name) {
                        writer.seek(SeekFrom::Start(tx_offset))?;
                        self.write_tx_block_raw(&mut writer, &ch.name)?;
                    }

                    // Write CN block
                    writer.seek(SeekFrom::Start(cn_offsets[dg_idx][cg_idx][cn_idx]))?;
                    let cn_next = if cn_idx + 1 < cn_offsets[dg_idx][cg_idx].len() { cn_offsets[dg_idx][cg_idx][cn_idx + 1] } else { 0 };

                    // Use cumulative byte offset for this channel
                    let byte_offset = cumulative_byte_offset;

                    self.write_cn_block_raw(&mut writer,
                        cn_next,
                        0,
                        *tx_offsets.get(&ch.name).unwrap_or(&0),
                        0,
                        0,
                        0,
                        0,
                        0,
                        0, // cn_type = fixed
                        0, // sync_type = none
                        ch.data_type,
                        0,
                        byte_offset,
                        ch.bit_count,
                    )?;

                    // Update cumulative offset for next channel
                    cumulative_byte_offset += ch.bit_count.div_ceil(8) as u32;
                    cn_idx += 1;
                }
            }
        }

        // === Write Data Blocks ===
        for (dg_idx, dg) in self.data_groups.iter().enumerate() {
            writer.seek(SeekFrom::Start(data_offsets[dg_idx]))?;

            let rec_id_size = dg.rec_id_size as usize;
            let mut all_data: Vec<u8> = Vec::new();

            // Process each channel group
            for cg in &dg.channel_groups {
                // Calculate record size for this CG
                let mut record_size: usize = 0;
                let master_byte_size = cg.master.as_ref().map(|m| m.bit_count.div_ceil(8) as usize).unwrap_or(0);
                record_size += master_byte_size;
                let channel_byte_sizes: Vec<usize> = cg.channels.iter().map(|ch| {
                    let size = ch.bit_count.div_ceil(8) as usize;
                    record_size += size;
                    size
                }).collect();

                // Get cycle count from master or first channel
                let cycle_count = if let Some(ref master) = cg.master {
                    self.channel_data.get(&master.name).map(|d| d.len() / master_byte_size).unwrap_or(0)
                } else if !cg.channels.is_empty() {
                    self.channel_data.get(&cg.channels[0].name).map(|d| d.len() / channel_byte_sizes[0]).unwrap_or(0)
                } else {
                    0
                };

                // Write each record with optional record ID prefix
                for cycle in 0..cycle_count {
                    // Write record ID prefix if needed (for multiple CGs)
                    if rec_id_size > 0 {
                        let rec_id = cg.record_id;
                        match rec_id_size {
                            1 => all_data.push(rec_id as u8),
                            2 => all_data.extend_from_slice(&(rec_id as u16).to_le_bytes()),
                            4 => all_data.extend_from_slice(&(rec_id as u32).to_le_bytes()),
                            8 => all_data.extend_from_slice(&rec_id.to_le_bytes()),
                            _ => {}
                        }
                    }

                    // Master channel first
                    if let Some(ref master) = cg.master {
                        if let Some(data) = self.channel_data.get(&master.name) {
                            let start = cycle * master_byte_size;
                            let end = start + master_byte_size;
                            if end <= data.len() {
                                all_data.extend_from_slice(&data[start..end]);
                            }
                        }
                    }
                    // Then regular channels
                    for (ch_idx, ch) in cg.channels.iter().enumerate() {
                        if let Some(data) = self.channel_data.get(&ch.name) {
                            let byte_size = channel_byte_sizes[ch_idx];
                            let start = cycle * byte_size;
                            let end = start + byte_size;
                            if end <= data.len() {
                                all_data.extend_from_slice(&data[start..end]);
                            }
                        }
                    }
                }
            }

            // Check if compression should be used
            #[cfg(feature = "compression")]
            let should_compress = self.compression.as_ref()
                .map(|c| all_data.len() as u64 >= c.min_size)
                .unwrap_or(false);

            #[cfg(not(feature = "compression"))]
            let should_compress = false;

            if should_compress {
                // Write DZ block (compressed) - only available with compression feature
                #[cfg(feature = "compression")]
                {
                    self.write_dz_block_raw(&mut writer, &all_data)?;
                }
                #[cfg(not(feature = "compression"))]
                {
                    self.write_dt_block_raw(&mut writer, &all_data)?;
                }
            } else {
                // Write DT block (uncompressed)
                self.write_dt_block_raw(&mut writer, &all_data)?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    // Helper methods for writing raw blocks
    fn write_id_block_raw<W: Write + Seek>(&self, writer: &mut W, id: &super::block_writer::IdBlock) -> WriteResult<()> {
        // ID block structure (64 bytes total, HD block starts at 0x40):
        // 0-7: id_file "MDF     " (8 bytes)
        // 8-15: id_ver version string (8 bytes)
        // 16-27: program identifier + padding (12 bytes)
        // 28-29: id_version version number (2 bytes)
        // 30-59: reserved (30 bytes)
        // 60-61: id_unfin_flags (2 bytes)
        // 62-63: id_custom_unfin_flags (2 bytes)

        writer.write_all(b"MDF     ")?;  // 0-7

        let mut ver_buf = [0u8; 8];
        let ver_bytes = id.id_ver.as_bytes();
        ver_buf[..ver_bytes.len().min(8)].copy_from_slice(&ver_bytes[..ver_bytes.len().min(8)]);
        writer.write_all(&ver_buf)?;  // 8-15

        // 12 bytes: program identifier + padding
        let mut program_buf = [0u8; 12];
        let program_bytes = id.id_program.as_bytes();
        program_buf[..program_bytes.len().min(12)].copy_from_slice(&program_bytes[..program_bytes.len().min(12)]);
        writer.write_all(&program_buf)?;  // 16-27

        writer.write_all(&id.id_version.to_le_bytes())?;  // 28-29
        writer.write_all(&[0u8; 30])?;  // 30-59 reserved
        writer.write_all(&id.id_unfin_flags.to_le_bytes())?;  // 60-61
        writer.write_all(&id.id_custom_unfin_flags.to_le_bytes())?;  // 62-63
        Ok(())
    }

    fn write_hd_block_raw<W: Write + Seek>(&self, writer: &mut W, hd: &super::block_writer::HdBlock) -> WriteResult<()> {
        // HD block structure according to MDF specification:
        // Links (6 total): hd_dg_first, hd_fh_first, hd_ch_first, hd_at_first, hd_ev_first, hd_md_comment
        // Data: hd_start_time_ns, hd_tz_offset_min, hd_dst_offset_min, hd_time_flags, hd_time_class, hd_flags, hd_reserved, hd_start_angle_rad, hd_start_distance_m

        writer.write_all(b"##HD")?;
        writer.write_all(&[0u8; 4])?;  // Reserved
        writer.write_all(&104u64.to_le_bytes())?; // Block length
        writer.write_all(&6u64.to_le_bytes())?;   // Link count (6 links)

        // Links (all 6, set to 0 if not used)
        writer.write_all(&hd.hd_dg_first.to_le_bytes())?;
        writer.write_all(&hd.hd_fh_first.to_le_bytes())?;
        writer.write_all(&0u64.to_le_bytes())?; // hd_ch_first
        writer.write_all(&0u64.to_le_bytes())?; // hd_at_first
        writer.write_all(&0u64.to_le_bytes())?; // hd_ev_first
        writer.write_all(&hd.hd_md_comment.to_le_bytes())?;

        // Data fields
        writer.write_all(&hd.hd_start_time_ns.to_le_bytes())?; // hd_start_time_ns (8 bytes)
        writer.write_all(&hd.hd_tz_offset.to_le_bytes())?;     // hd_tz_offset_min (2 bytes)
        writer.write_all(&hd.hd_dst_offset.to_le_bytes())?;    // hd_dst_offset_min (2 bytes)
        writer.write_all(&hd.hd_time_flags.to_le_bytes())?;    // hd_time_flags (1 byte)
        writer.write_all(&[0u8; 3])?;  // hd_time_class, hd_flags, hd_reserved (3 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // hd_start_angle_rad (8 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // hd_start_distance_m (8 bytes)
        Ok(())
    }

    fn write_dg_block_raw<W: Write + Seek>(&self, writer: &mut W, dg_next: u64, cg_first: u64, data: u64, md_comment: u64, rec_id_size: u8) -> WriteResult<()> {
        writer.write_all(b"##DG")?;
        writer.write_all(&[0u8; 4])?;
        writer.write_all(&64u64.to_le_bytes())?;
        writer.write_all(&4u64.to_le_bytes())?;
        writer.write_all(&dg_next.to_le_bytes())?;
        writer.write_all(&cg_first.to_le_bytes())?;
        writer.write_all(&data.to_le_bytes())?;
        writer.write_all(&md_comment.to_le_bytes())?;
        writer.write_all(&rec_id_size.to_le_bytes())?;
        writer.write_all(&[0u8; 7])?;
        Ok(())
    }

    fn write_cg_block_raw<W: Write + Seek>(&self, writer: &mut W, cg_next: u64, cn_first: u64, tx_acq_name: u64, si_acq_source: u64, md_comment: u64, record_id: u64, cycle_count: u64, data_bytes: u32, inval_bytes: u32) -> WriteResult<()> {
        // CG block structure:
        // Links (6): cg_cg_next, cg_cn_first, cg_tx_acq_name, cg_si_acq_source, cg_cg_master, cg_md_comment
        // Data: cg_record_id (8), cg_cycle_count (8), cg_flags (2), cg_path_separator (2), cg_reserved (4), cg_data_bytes (4), cg_inval_bytes (4)
        // Total: 24 + 48 + 32 = 104 bytes

        writer.write_all(b"##CG")?;
        writer.write_all(&[0u8; 4])?;
        writer.write_all(&104u64.to_le_bytes())?; // Block length
        writer.write_all(&6u64.to_le_bytes())?;   // Link count (6 links)

        // Links
        writer.write_all(&cg_next.to_le_bytes())?;
        writer.write_all(&cn_first.to_le_bytes())?;
        writer.write_all(&tx_acq_name.to_le_bytes())?;
        writer.write_all(&si_acq_source.to_le_bytes())?;
        writer.write_all(&0u64.to_le_bytes())?;    // cg_cg_master (optional, 0 if not used)
        writer.write_all(&md_comment.to_le_bytes())?;

        // Data fields (in correct order)
        writer.write_all(&record_id.to_le_bytes())?;   // cg_record_id (8 bytes)
        writer.write_all(&cycle_count.to_le_bytes())?; // cg_cycle_count (8 bytes)
        writer.write_all(&0u16.to_le_bytes())?;        // cg_flags (2 bytes)
        writer.write_all(&0u16.to_le_bytes())?;        // cg_path_separator (2 bytes)
        writer.write_all(&[0u8; 4])?;                   // cg_reserved (4 bytes)
        writer.write_all(&data_bytes.to_le_bytes())?; // cg_data_bytes (4 bytes)
        writer.write_all(&inval_bytes.to_le_bytes())?; // cg_inval_bytes (4 bytes)
        Ok(())
    }

    fn write_cn_block_raw<W: Write + Seek>(&self, writer: &mut W, cn_next: u64, composition: u64, tx_name: u64, si_source: u64, cc_conversion: u64, cn_data: u64, md_unit: u64, md_comment: u64, cn_type: u8, sync_type: u8, data_type: u8, bit_offset: u8, byte_offset: u32, bit_count: u32) -> WriteResult<()> {
        // CN block structure:
        // Links (8): cn_cn_next, cn_composition, cn_tx_name, cn_si_source, cn_cc_conversion, cn_data, cn_md_unit, cn_md_comment
        // Data: cn_type, cn_sync_type, cn_data_type, cn_bit_offset, cn_byte_offset, cn_bit_count, cn_flags, cn_inval_bit_pos,
        //       cn_precision, cn_reserved, cn_attachment_count, cn_val_range_min, cn_val_range_max,
        //       cn_limit_min, cn_limit_max, cn_limit_ext_min, cn_limit_ext_max
        // Total: 24 + 64 + 72 = 160 bytes

        writer.write_all(b"##CN")?;
        writer.write_all(&[0u8; 4])?;
        writer.write_all(&160u64.to_le_bytes())?; // Block length
        writer.write_all(&8u64.to_le_bytes())?;   // Link count

        // Links (8)
        writer.write_all(&cn_next.to_le_bytes())?;
        writer.write_all(&composition.to_le_bytes())?;
        writer.write_all(&tx_name.to_le_bytes())?;
        writer.write_all(&si_source.to_le_bytes())?;
        writer.write_all(&cc_conversion.to_le_bytes())?;
        writer.write_all(&cn_data.to_le_bytes())?;
        writer.write_all(&md_unit.to_le_bytes())?;
        writer.write_all(&md_comment.to_le_bytes())?;

        // Data fields (in correct order)
        writer.write_all(&[cn_type])?;           // cn_type (1 byte)
        writer.write_all(&[sync_type])?;         // cn_sync_type (1 byte)
        writer.write_all(&[data_type])?;         // cn_data_type (1 byte)
        writer.write_all(&[bit_offset])?;        // cn_bit_offset (1 byte)
        writer.write_all(&byte_offset.to_le_bytes())?; // cn_byte_offset (4 bytes)
        writer.write_all(&bit_count.to_le_bytes())?; // cn_bit_count (4 bytes)
        writer.write_all(&0u32.to_le_bytes())?;  // cn_flags (4 bytes)
        writer.write_all(&0u32.to_le_bytes())?;  // cn_inval_bit_pos (4 bytes)
        writer.write_all(&[0u8])?;               // cn_precision (1 byte)
        writer.write_all(&[0u8])?;               // cn_reserved (1 byte)
        writer.write_all(&0u16.to_le_bytes())?; // cn_attachment_count (2 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // cn_val_range_min (8 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // cn_val_range_max (8 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // cn_limit_min (8 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // cn_limit_max (8 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // cn_limit_ext_min (8 bytes)
        writer.write_all(&0f64.to_le_bytes())?; // cn_limit_ext_max (8 bytes)
        Ok(())
    }

    fn write_tx_block_raw<W: Write + Seek>(&self, writer: &mut W, text: &str) -> WriteResult<()> {
        let text_bytes = text.as_bytes();
        let text_len = text_bytes.len() + 1; // Include null terminator
        let padded_len = (text_len + 7) & !7; // Pad to 8-byte boundary
        let block_len = 24 + padded_len as u64;

        writer.write_all(b"##TX")?;
        writer.write_all(&[0u8; 4])?;  // Reserved
        writer.write_all(&block_len.to_le_bytes())?;
        writer.write_all(&0u64.to_le_bytes())?;  // Link count

        // Write text data
        writer.write_all(text_bytes)?;
        writer.write_all(&[0u8])?;  // Null terminator

        // Padding (max 7 bytes)
        let padding = padded_len - text_len;
        if padding > 0 {
            const ZEROS: [u8; 7] = [0u8; 7];
            writer.write_all(&ZEROS[..padding])?;
        }

        Ok(())
    }

    fn write_dt_block_raw<W: Write + Seek>(&self, writer: &mut W, data: &[u8]) -> WriteResult<()> {
        let block_len = 24u64 + data.len() as u64;
        writer.write_all(b"##DT")?;
        writer.write_all(&[0u8; 4])?;
        writer.write_all(&block_len.to_le_bytes())?;
        writer.write_all(&0u64.to_le_bytes())?;
        writer.write_all(data)?;
        Ok(())
    }

    #[cfg(feature = "compression")]
    fn write_dz_block_raw<W: Write + Seek>(&self, writer: &mut W, data: &[u8]) -> WriteResult<()> {
        // Compress the data
        let compression_config = self.compression.as_ref().unwrap();
        let compressor = super::compression::Compressor {
            compression_type: if compression_config.zip_type == 0 {
                super::compression::CompressionType::Deflate
            } else {
                super::compression::CompressionType::TransposeDeflate
            },
            level: compression_config.level,
            column_count: None,
        };

        let (compressed_data, original_len) = compressor.compress(data)?;

        // DZ block structure
        let block_len = 24u64 + 8 + 8 + 1 + 3 + 4 + compressed_data.len() as u64;

        writer.write_all(b"##DZ")?;
        writer.write_all(&[0u8; 4])?;
        writer.write_all(&block_len.to_le_bytes())?;
        writer.write_all(&1u64.to_le_bytes())?;  // Link count (dz_data)

        // Data offset (position after this header + the link itself)
        let current_pos = writer.stream_position()?;
        let data_offset = current_pos + 8 + 8 + 1 + 3 + 4; // remaining header bytes
        writer.write_all(&data_offset.to_le_bytes())?;

        // DZ data fields
        writer.write_all(&original_len.to_le_bytes())?;  // dz_org_data_length
        writer.write_all(&(compressed_data.len() as u64).to_le_bytes())?;  // dz_data_length
        writer.write_all(&compression_config.zip_type.to_le_bytes())?;  // dz_zip_type
        writer.write_all(&[0u8; 3])?;  // reserved
        writer.write_all(&0u32.to_le_bytes())?;  // dz_zip_parameter

        // Compressed data
        writer.write_all(&compressed_data)?;

        Ok(())
    }

    /// Get the number of data groups
    pub fn data_group_count(&self) -> usize {
        self.data_groups.len()
    }

    /// Get the metadata
    pub fn metadata(&self) -> &Mf4Metadata {
        &self.metadata
    }
}

// ============================================================================
// Channel Data Serialization Trait
// ============================================================================

/// Trait for serializing channel data to bytes
pub trait ChannelData: Sized + Clone + 'static {
    /// Serialize data to bytes with the specified data type and bit count
    ///
    /// # Arguments
    /// * `data` - Slice of values to serialize
    /// * `data_type` - MDF data type (0=UINT_LE, 1=UINT_BE, 2=INT_LE, 3=INT_BE, 4=FLOAT_LE, 5=FLOAT_BE)
    /// * `bit_count` - Number of bits per value (8, 16, 32, 64)
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>>;

    /// Get the MDF data type code for this type
    fn default_data_type() -> u8;

    /// Get the default bit count for this type
    fn default_bit_count() -> u32;
}

impl ChannelData for f64 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        match data_type {
            4 | 5 => { // FLOAT_LE or FLOAT_BE
                if bit_count != 64 {
                    return Err(WriteError::InvalidChannelConfig(
                        format!("f64 requires bit_count=64, got {}", bit_count)
                    ));
                }
                let mut result = Vec::with_capacity(data.len() * 8);
                for value in data {
                    if data_type == 4 {
                        result.extend_from_slice(&value.to_le_bytes());
                    } else {
                        result.extend_from_slice(&value.to_be_bytes());
                    }
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "f64".to_string(),
                expected: 4,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 5 } // FLOAT_LE
    fn default_bit_count() -> u32 { 64 }
}

impl ChannelData for f32 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        match data_type {
            4 | 5 => {
                if bit_count != 32 {
                    return Err(WriteError::InvalidChannelConfig(
                        format!("f32 requires bit_count=32, got {}", bit_count)
                    ));
                }
                let mut result = Vec::with_capacity(data.len() * 4);
                for value in data {
                    if data_type == 4 {
                        result.extend_from_slice(&value.to_le_bytes());
                    } else {
                        result.extend_from_slice(&value.to_be_bytes());
                    }
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "f32".to_string(),
                expected: 4,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 4 } // FLOAT_LE
    fn default_bit_count() -> u32 { 32 }
}

impl ChannelData for u8 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 8 {
            return Err(WriteError::InvalidChannelConfig(
                format!("u8 requires bit_count=8, got {}", bit_count)
            ));
        }
        match data_type {
            0 | 1 => Ok(data.to_vec()), // UINT_LE or UINT_BE (same for u8)
            _ => Err(WriteError::InvalidDataType {
                channel: "u8".to_string(),
                expected: 0,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 0 } // UINT_LE
    fn default_bit_count() -> u32 { 8 }
}

impl ChannelData for u16 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 16 {
            return Err(WriteError::InvalidChannelConfig(
                format!("u16 requires bit_count=16, got {}", bit_count)
            ));
        }
        match data_type {
            0 => { // UINT_LE
                let mut result = Vec::with_capacity(data.len() * 2);
                for value in data {
                    result.extend_from_slice(&value.to_le_bytes());
                }
                Ok(result)
            }
            1 => { // UINT_BE
                let mut result = Vec::with_capacity(data.len() * 2);
                for value in data {
                    result.extend_from_slice(&value.to_be_bytes());
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "u16".to_string(),
                expected: 0,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 0 }
    fn default_bit_count() -> u32 { 16 }
}

impl ChannelData for u32 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 32 {
            return Err(WriteError::InvalidChannelConfig(
                format!("u32 requires bit_count=32, got {}", bit_count)
            ));
        }
        match data_type {
            0 => {
                let mut result = Vec::with_capacity(data.len() * 4);
                for value in data {
                    result.extend_from_slice(&value.to_le_bytes());
                }
                Ok(result)
            }
            1 => {
                let mut result = Vec::with_capacity(data.len() * 4);
                for value in data {
                    result.extend_from_slice(&value.to_be_bytes());
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "u32".to_string(),
                expected: 0,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 0 }
    fn default_bit_count() -> u32 { 32 }
}

impl ChannelData for u64 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 64 {
            return Err(WriteError::InvalidChannelConfig(
                format!("u64 requires bit_count=64, got {}", bit_count)
            ));
        }
        match data_type {
            0 => {
                let mut result = Vec::with_capacity(data.len() * 8);
                for value in data {
                    result.extend_from_slice(&value.to_le_bytes());
                }
                Ok(result)
            }
            1 => {
                let mut result = Vec::with_capacity(data.len() * 8);
                for value in data {
                    result.extend_from_slice(&value.to_be_bytes());
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "u64".to_string(),
                expected: 0,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 0 }
    fn default_bit_count() -> u32 { 64 }
}

impl ChannelData for i8 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 8 {
            return Err(WriteError::InvalidChannelConfig(
                format!("i8 requires bit_count=8, got {}", bit_count)
            ));
        }
        match data_type {
            2 | 3 => Ok(data.iter().map(|v| *v as u8).collect()),
            _ => Err(WriteError::InvalidDataType {
                channel: "i8".to_string(),
                expected: 2,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 2 } // INT_LE
    fn default_bit_count() -> u32 { 8 }
}

impl ChannelData for i16 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 16 {
            return Err(WriteError::InvalidChannelConfig(
                format!("i16 requires bit_count=16, got {}", bit_count)
            ));
        }
        match data_type {
            2 => {
                let mut result = Vec::with_capacity(data.len() * 2);
                for value in data {
                    result.extend_from_slice(&value.to_le_bytes());
                }
                Ok(result)
            }
            3 => {
                let mut result = Vec::with_capacity(data.len() * 2);
                for value in data {
                    result.extend_from_slice(&value.to_be_bytes());
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "i16".to_string(),
                expected: 2,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 2 }
    fn default_bit_count() -> u32 { 16 }
}

impl ChannelData for i32 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 32 {
            return Err(WriteError::InvalidChannelConfig(
                format!("i32 requires bit_count=32, got {}", bit_count)
            ));
        }
        match data_type {
            2 => {
                let mut result = Vec::with_capacity(data.len() * 4);
                for value in data {
                    result.extend_from_slice(&value.to_le_bytes());
                }
                Ok(result)
            }
            3 => {
                let mut result = Vec::with_capacity(data.len() * 4);
                for value in data {
                    result.extend_from_slice(&value.to_be_bytes());
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "i32".to_string(),
                expected: 2,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 2 }
    fn default_bit_count() -> u32 { 32 }
}

impl ChannelData for i64 {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        if bit_count != 64 {
            return Err(WriteError::InvalidChannelConfig(
                format!("i64 requires bit_count=64, got {}", bit_count)
            ));
        }
        match data_type {
            2 => {
                let mut result = Vec::with_capacity(data.len() * 8);
                for value in data {
                    result.extend_from_slice(&value.to_le_bytes());
                }
                Ok(result)
            }
            3 => {
                let mut result = Vec::with_capacity(data.len() * 8);
                for value in data {
                    result.extend_from_slice(&value.to_be_bytes());
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "i64".to_string(),
                expected: 2,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 2 }
    fn default_bit_count() -> u32 { 64 }
}

// ============================================================================
// String Channel Data Support
// ============================================================================

/// Fixed-length string channel data (UTF-8)
///
/// For variable-length strings, VLSD channels should be used instead.
/// This implementation writes fixed-length strings padded with null bytes.
impl ChannelData for String {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        // Data types 6 = STRING_LE (UTF-8), 7 = STRING_BE (UTF-8)
        // Data types 8 = STRING_UTF16_LE, 9 = STRING_UTF16_BE
        let max_len = (bit_count / 8) as usize;

        match data_type {
            6 | 7 => {
                // UTF-8 strings
                let mut result = Vec::with_capacity(data.len() * max_len);
                for s in data {
                    let bytes = s.as_bytes();
                    let len = bytes.len().min(max_len);
                    result.extend_from_slice(&bytes[..len]);
                    // Pad with null bytes if string is shorter than max_len
                    if len < max_len {
                        result.extend(std::iter::repeat_n(0u8, max_len - len));
                    }
                }
                Ok(result)
            }
            8 => {
                // UTF-16 LE
                let mut result = Vec::with_capacity(data.len() * max_len);
                for s in data {
                    let encoded: Vec<u16> = s.encode_utf16().collect();
                    let max_chars = max_len / 2;
                    for ch in encoded.iter().take(max_chars) {
                        result.extend_from_slice(&ch.to_le_bytes());
                    }
                    // Pad remaining with null
                    let written = encoded.len().min(max_chars);
                    if written < max_chars {
                        result.extend(std::iter::repeat_n(0u8, (max_chars - written) * 2));
                    }
                }
                Ok(result)
            }
            9 => {
                // UTF-16 BE
                let mut result = Vec::with_capacity(data.len() * max_len);
                for s in data {
                    let encoded: Vec<u16> = s.encode_utf16().collect();
                    let max_chars = max_len / 2;
                    for ch in encoded.iter().take(max_chars) {
                        result.extend_from_slice(&ch.to_be_bytes());
                    }
                    // Pad remaining with null
                    let written = encoded.len().min(max_chars);
                    if written < max_chars {
                        result.extend(std::iter::repeat_n(0u8, (max_chars - written) * 2));
                    }
                }
                Ok(result)
            }
            _ => Err(WriteError::InvalidDataType {
                channel: "String".to_string(),
                expected: 6,
                actual: data_type,
            }),
        }
    }

    fn default_data_type() -> u8 { 6 } // STRING_LE (UTF-8)
    fn default_bit_count() -> u32 { 256 } // Default 256 bits = 32 bytes
}

/// Byte array channel data
impl ChannelData for Vec<u8> {
    fn serialize_to_bytes(data: &[Self], data_type: u8, bit_count: u32) -> WriteResult<Vec<u8>> {
        // Data type 10 = BYTE_ARRAY
        if data_type != 10 {
            return Err(WriteError::InvalidDataType {
                channel: "Vec<u8>".to_string(),
                expected: 10,
                actual: data_type,
            });
        }

        let array_len = (bit_count / 8) as usize;
        let mut result = Vec::with_capacity(data.len() * array_len);

        for arr in data {
            let len = arr.len().min(array_len);
            result.extend_from_slice(&arr[..len]);
            // Pad with zeros if array is shorter
            if len < array_len {
                result.extend(std::iter::repeat_n(0u8, array_len - len));
            }
        }

        Ok(result)
    }

    fn default_data_type() -> u8 { 10 }
    fn default_bit_count() -> u32 { 64 } // Default 64 bits = 8 bytes
}
