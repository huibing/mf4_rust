//! BlockWriter - Low-level block writing primitives
//!
//! This module provides low-level functions for writing MF4 blocks directly to a file.

use std::io::{Seek, SeekFrom, Write};

use super::error::WriteResult;

// ============================================================================
// Block Header Constants
// ============================================================================

/// Block header size (ID + reserved + length + link_count)
pub const BLOCK_HEADER_SIZE: u64 = 24;

/// Block ID strings
pub mod block_id {
    pub const ID: &[u8; 8] = b"MDF     ";
    pub const HD: &[u8; 4] = b"##HD";
    pub const FH: &[u8; 4] = b"##FH";
    pub const DG: &[u8; 4] = b"##DG";
    pub const CG: &[u8; 4] = b"##CG";
    pub const CN: &[u8; 4] = b"##CN";
    pub const CC: &[u8; 4] = b"##CC";
    pub const CA: &[u8; 4] = b"##CA";
    pub const DT: &[u8; 4] = b"##DT";
    pub const SD: &[u8; 4] = b"##SD";
    pub const RD: &[u8; 4] = b"##RD";
    pub const DZ: &[u8; 4] = b"##DZ";
    pub const DL: &[u8; 4] = b"##DL";
    pub const HL: &[u8; 4] = b"##HL";
    pub const TX: &[u8; 4] = b"##TX";
    pub const MD: &[u8; 4] = b"##MD";
    pub const SI: &[u8; 4] = b"##SI";
}

// ============================================================================
// ID Block
// ============================================================================

/// ID Block (File Identification)
#[derive(Debug, Clone)]
pub struct IdBlock {
    /// File identifier (always "MDF     ")
    pub id_file: String,
    /// Version string (e.g., "4.10    ")
    pub id_ver: String,
    /// Program identifier
    pub id_program: String,
    /// Version number (e.g., 410)
    pub id_version: u16,
    /// Unfinalized flags
    pub id_unfin_flags: u16,
    /// Custom unfinalized flags
    pub id_custom_unfin_flags: u16,
}

impl Default for IdBlock {
    fn default() -> Self {
        Self {
            id_file: "MDF     ".to_string(),
            id_ver: "4.10    ".to_string(),
            id_program: "Mf4Parse".to_string(),
            id_version: 410,
            id_unfin_flags: 0,
            id_custom_unfin_flags: 0,
        }
    }
}

// ============================================================================
// HD Block
// ============================================================================

/// HD Block (Header)
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct HdBlock {
    /// Pointer to first DG block
    pub hd_dg_first: u64,
    /// Pointer to file history FH block
    pub hd_fh_first: u64,
    /// Pointer to comment MD block
    pub hd_md_comment: u64,
    /// Start time in nanoseconds since 1970-01-01
    pub hd_start_time_ns: u64,
    /// Time zone offset in minutes (0 = UTC)
    pub hd_tz_offset: i16,
    /// Daylight saving time offset in minutes
    pub hd_dst_offset: i16,
    /// Time flags
    pub hd_time_flags: u8,
    /// Time quality
    pub hd_time_quality: u8,
    /// Number of timers
    pub hd_num_time_channels: u32,
}


// ============================================================================
// FH Block (File History)
// ============================================================================

/// FH Block (File History) - Mandatory for MDF 4.x
#[derive(Debug, Clone)]
pub struct FhBlock {
    /// Pointer to next FH block
    pub fh_fh_next: u64,
    /// Pointer to comment MD block
    pub fh_md_comment: u64,
    /// Time stamp in nanoseconds since 1970-01-01
    pub fh_time_ns: u64,
    /// Time zone offset in minutes
    pub fh_tz_offset: i16,
    /// Daylight saving time offset in minutes
    pub fh_dst_offset: i16,
    /// Tool ID (e.g., "Mf4Parse")
    pub fh_tool_id: String,
    /// Tool vendor
    pub fh_tool_vendor: String,
    /// Tool version
    pub fh_tool_version: String,
    /// User name
    pub fh_user_name: String,
}

impl Default for FhBlock {
    fn default() -> Self {
        Self {
            fh_fh_next: 0,
            fh_md_comment: 0,
            fh_time_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            fh_tz_offset: 0,
            fh_dst_offset: 0,
            fh_tool_id: "Mf4Parse".to_string(),
            fh_tool_vendor: "".to_string(),
            fh_tool_version: env!("CARGO_PKG_VERSION").to_string(),
            fh_user_name: "".to_string(),
        }
    }
}

// ============================================================================
// DG Block
// ============================================================================

/// DG Block (Data Group)
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct DgBlock {
    /// Pointer to next DG block
    pub dg_dg_next: u64,
    /// Pointer to first CG block
    pub dg_cg_first: u64,
    /// Pointer to data block (DT/DZ/DL/HL)
    pub dg_data: u64,
    /// Pointer to comment MD block
    pub dg_md_comment: u64,
    /// Record ID size (0, 1, 2, 4, 8)
    pub dg_rec_id_size: u8,
}


// ============================================================================
// CG Block
// ============================================================================

/// CG Block (Channel Group)
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct CgBlock {
    /// Pointer to next CG block
    pub cg_cg_next: u64,
    /// Pointer to first CN block
    pub cg_cn_first: u64,
    /// Pointer to acquisition name TX block
    pub cg_tx_acq_name: u64,
    /// Pointer to acquisition source SI block
    pub cg_si_acq_source: u64,
    /// Pointer to comment MD block
    pub cg_md_comment: u64,
    /// Record ID
    pub cg_record_id: u64,
    /// Cycle count
    pub cg_cycle_count: u64,
    /// Data bytes per record
    pub cg_data_bytes: u32,
    /// Invalid bytes per record
    pub cg_inval_bytes: u32,
    /// Flags
    pub cg_flags: u16,
    /// Path separator
    pub cg_path_separator: u16,
    /// Number of samples (for fixed length)
    pub cg_samples: u32,
}


// ============================================================================
// CN Block
// ============================================================================

/// CN Block (Channel)
#[derive(Debug, Clone)]
pub struct CnBlock {
    /// Pointer to next CN block
    pub cn_cn_next: u64,
    /// Pointer to composition CN/CA block
    pub cn_composition: u64,
    /// Pointer to TX block for channel name
    pub cn_tx_name: u64,
    /// Pointer to SI block for source
    pub cn_si_source: u64,
    /// Pointer to CC block for conversion
    pub cn_cc_conversion: u64,
    /// Pointer to data block (for VLSD)
    pub cn_data: u64,
    /// Pointer to unit MD block
    pub cn_md_unit: u64,
    /// Pointer to comment MD block
    pub cn_md_comment: u64,
    /// Channel type (0=Fixed, 1=VLSD, 2=Master, 3=VirtualMaster)
    pub cn_type: u8,
    /// Sync type (0=None, 1=Time, 2=Angle, 3=Distance, 4=Index)
    pub cn_sync_type: u8,
    /// Data type (0-10)
    pub cn_data_type: u8,
    /// Bit offset
    pub cn_bit_offset: u8,
    /// Byte offset
    pub cn_byte_offset: u32,
    /// Bit count
    pub cn_bit_count: u32,
    /// Flags
    pub cn_flags: u32,
    /// Invalid bit position
    pub cn_inval_bit_pos: u32,
    /// Attachment count
    pub cn_attachment_count: u16,
    /// Precision for floating point
    pub cn_precision: u8,
    /// Value limit 1 (min)
    pub cn_val_limit_1: f64,
    /// Value limit 2 (max)
    pub cn_val_limit_2: f64,
}

impl Default for CnBlock {
    fn default() -> Self {
        Self {
            cn_cn_next: 0,
            cn_composition: 0,
            cn_tx_name: 0,
            cn_si_source: 0,
            cn_cc_conversion: 0,
            cn_data: 0,
            cn_md_unit: 0,
            cn_md_comment: 0,
            cn_type: 0,
            cn_sync_type: 0,
            cn_data_type: 0,
            cn_bit_offset: 0,
            cn_byte_offset: 0,
            cn_bit_count: 8,
            cn_flags: 0,
            cn_inval_bit_pos: 0,
            cn_attachment_count: 0,
            cn_precision: 0,
            cn_val_limit_1: 0.0,
            cn_val_limit_2: 0.0,
        }
    }
}

// ============================================================================
// TX Block
// ============================================================================

/// TX Block (Text)
#[derive(Debug, Clone)]
pub struct TxBlock {
    /// Text data (null-terminated)
    pub tx_data: String,
}

impl TxBlock {
    /// Create a new TX block
    pub fn new(text: &str) -> Self {
        Self {
            tx_data: text.to_string(),
        }
    }
}

// ============================================================================
// DT Block
// ============================================================================

/// DT Block (Data)
#[derive(Debug, Clone)]
pub struct DtBlock {
    /// Raw data
    pub data: Vec<u8>,
}

impl DtBlock {
    /// Create a new DT block
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

// ============================================================================
// DZ Block
// ============================================================================

/// DZ Block (Compressed Data)
#[derive(Debug, Clone)]
pub struct DzBlock {
    /// Original (uncompressed) data length
    pub dz_org_data_length: u64,
    /// Compressed data length
    pub dz_data_length: u64,
    /// Compression type (0=Deflate, 1=Transpose+Deflate)
    pub dz_zip_type: u8,
    /// Compression parameter (column count for transpose)
    pub dz_zip_parameter: u32,
    /// Compressed data
    pub data: Vec<u8>,
}

// ============================================================================
// HL Block (Header List)
// ============================================================================

/// HL Block (Header List for compressed data with DL chain)
///
/// When DZ blocks are linked via DL, the DG must point to an HL block,
/// which then points to the first DL block. This is required by the MDF4 spec.
#[derive(Debug, Clone)]
pub struct HlBlock {
    /// Offset to first DL block
    pub hl_dl_first: u64,
    /// Flags (UINT16)
    pub hl_flags: u16,
    /// Compression type: 0=Deflate, 1=Transpose+Deflate
    pub hl_zip_type: u8,
}

// ============================================================================
// CC Block (Conversion)
// ============================================================================

/// CC Block (Conversion rules for channel data)
#[derive(Debug, Clone)]
pub struct CcBlock {
    /// Pointer to name TX block
    pub cc_tx_name: u64,
    /// Pointer to comment MD block
    pub cc_md_comment: u64,
    /// Pointer to inverse conversion CC block
    pub cc_cc_inverse: u64,
    /// Pointer to unit TX block
    pub cc_tx_unit: u64,
    /// Conversion type (0=1:1, 1=Linear, 2=Rational, 3=Algebraic, 4=TableInt, 5=Table, etc.)
    pub cc_type: u8,
    /// Precision display
    pub cc_precision: u8,
    /// Flags
    pub cc_flags: u16,
    /// Reference count (number of ref links)
    pub cc_ref_count: u16,
    /// Value count (number of val values)
    pub cc_val_count: u16,
    /// Minimum physical signal value
    pub cc_phy_range_min: f64,
    /// Maximum physical signal value
    pub cc_phy_range_max: f64,
    /// Conversion values (depends on cc_type)
    pub cc_val: Vec<u64>,
    /// Reference links (TX or CC blocks for text conversions)
    pub cc_ref: Vec<u64>,
}

impl Default for CcBlock {
    fn default() -> Self {
        Self {
            cc_tx_name: 0,
            cc_md_comment: 0,
            cc_cc_inverse: 0,
            cc_tx_unit: 0,
            cc_type: 0, // 1:1 conversion
            cc_precision: 0,
            cc_flags: 0,
            cc_ref_count: 0,
            cc_val_count: 0,
            cc_phy_range_min: 0.0,
            cc_phy_range_max: 0.0,
            cc_val: Vec::new(),
            cc_ref: Vec::new(),
        }
    }
}

impl CcBlock {
    /// Create a new CC block with default (1:1) conversion
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a linear conversion: y = p1 + p2 * x
    pub fn linear(p1: f64, p2: f64) -> Self {
        Self {
            cc_type: 1,
            cc_val: vec![p1.to_bits(), p2.to_bits()],
            cc_val_count: 2,
            ..Self::default()
        }
    }
}

// ============================================================================
// SI Block (Source Information)
// ============================================================================

/// SI Block (Source information for channels and channel groups)
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct SiBlock {
    /// Pointer to name TX block
    pub si_tx_name: u64,
    /// Pointer to path TX block
    pub si_tx_path: u64,
    /// Pointer to comment MD block
    pub si_md_comment: u64,
    /// Source type (0=Other, 1=ECU, 2=Bus, 3=I/O, 4=Tool, 5=User)
    pub si_type: u8,
    /// Bus type (0=None, 1=Other, 2=CAN, 3=LIN, 4=FlexRay, 5=Most, 6=Ethernet)
    pub si_bus_type: u8,
    /// Flags
    pub si_flags: u8,
}


impl SiBlock {
    /// Create a new SI block
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an SI block for ECU source
    pub fn ecu() -> Self {
        Self {
            si_type: 1, // ECU
            ..Self::default()
        }
    }
}

// ============================================================================
// BlockWriter
// ============================================================================

/// Low-level block writer for MF4 files
pub struct BlockWriter<'a, W: Write + Seek> {
    writer: &'a mut W,
    current_offset: u64,
}

impl<'a, W: Write + Seek> BlockWriter<'a, W> {
    /// Create a new BlockWriter
    pub fn new(writer: &'a mut W) -> WriteResult<Self> {
        let current_offset = writer.stream_position()?;
        Ok(Self { writer, current_offset })
    }

    /// Get current file position
    pub fn position(&self) -> u64 {
        self.current_offset
    }

    /// Seek to a specific offset
    pub fn seek(&mut self, offset: u64) -> WriteResult<()> {
        self.writer.seek(SeekFrom::Start(offset))?;
        self.current_offset = offset;
        Ok(())
    }

    /// Align to 8-byte boundary
    pub fn align_to_8(&mut self) -> WriteResult<u64> {
        let remainder = self.current_offset % 8;
        if remainder != 0 {
            let padding = (8 - remainder) as usize;
            // Use stack-allocated array for small padding (max 7 bytes)
            const ZEROS: [u8; 7] = [0u8; 7];
            self.writer.write_all(&ZEROS[..padding])?;
            self.current_offset += padding as u64;
        }
        Ok(self.current_offset)
    }

    /// Write ID block (file identification)
    ///
    /// ID block structure (64 bytes total, HD block starts at 0x40):
    /// - 0-7: id_file "MDF     " (8 bytes)
    /// - 8-15: id_ver version string (8 bytes)
    /// - 16-27: program identifier + padding (12 bytes)
    /// - 28-29: id_version version number (2 bytes)
    /// - 30-59: reserved (30 bytes)
    /// - 60-61: id_unfin_flags (2 bytes)
    /// - 62-63: id_custom_unfin_flags (2 bytes)
    pub fn write_id_block(&mut self, id: &IdBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // id_file (8 bytes)
        self.writer.write_all(id.id_file.as_bytes())?;

        // id_ver (8 bytes)
        let mut ver_buf = [0u8; 8];
        let ver_bytes = id.id_ver.as_bytes();
        ver_buf[..ver_bytes.len().min(8)].copy_from_slice(&ver_bytes[..ver_bytes.len().min(8)]);
        self.writer.write_all(&ver_buf)?;

        // Program identifier + padding (12 bytes)
        let mut program_buf = [0u8; 12];
        let program_bytes = id.id_program.as_bytes();
        program_buf[..program_bytes.len().min(12)].copy_from_slice(&program_bytes[..program_bytes.len().min(12)]);
        self.writer.write_all(&program_buf)?;

        // id_version (2 bytes)
        self.writer.write_all(&id.id_version.to_le_bytes())?;

        // Reserved (30 bytes)
        self.writer.write_all(&[0u8; 30])?;

        // id_unfin_flags (2 bytes)
        self.writer.write_all(&id.id_unfin_flags.to_le_bytes())?;

        // id_custom_unfin_flags (2 bytes)
        self.writer.write_all(&id.id_custom_unfin_flags.to_le_bytes())?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write HD block (header)
    ///
    /// HD block structure according to MDF specification:
    /// - Links (6): hd_dg_first, hd_fh_first, hd_ch_first, hd_at_first, hd_ev_first, hd_md_comment
    /// - Data: hd_start_time_ns, hd_tz_offset_min, hd_dst_offset_min, hd_time_flags, hd_time_class, hd_flags, hd_reserved, hd_start_angle_rad, hd_start_distance_m
    pub fn write_hd_block(&mut self, hd: &HdBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // Block ID
        self.writer.write_all(block_id::HD)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length (8 bytes) - 104 bytes
        self.writer.write_all(&104u64.to_le_bytes())?;
        // Link count (6 links)
        self.writer.write_all(&6u64.to_le_bytes())?;

        // Links (all 6, set to 0 if not used)
        self.writer.write_all(&hd.hd_dg_first.to_le_bytes())?;
        self.writer.write_all(&hd.hd_fh_first.to_le_bytes())?;
        self.writer.write_all(&0u64.to_le_bytes())?; // hd_ch_first
        self.writer.write_all(&0u64.to_le_bytes())?; // hd_at_first
        self.writer.write_all(&0u64.to_le_bytes())?; // hd_ev_first
        self.writer.write_all(&hd.hd_md_comment.to_le_bytes())?;

        // Data fields
        self.writer.write_all(&hd.hd_start_time_ns.to_le_bytes())?; // hd_start_time_ns (8 bytes)
        self.writer.write_all(&hd.hd_tz_offset.to_le_bytes())?;     // hd_tz_offset_min (2 bytes)
        self.writer.write_all(&hd.hd_dst_offset.to_le_bytes())?;    // hd_dst_offset_min (2 bytes)
        self.writer.write_all(&hd.hd_time_flags.to_le_bytes())?;    // hd_time_flags (1 byte)
        self.writer.write_all(&[0u8; 3])?;  // hd_time_class, hd_flags, hd_reserved (3 bytes)
        self.writer.write_all(&0f64.to_le_bytes())?; // hd_start_angle_rad (8 bytes)
        self.writer.write_all(&0f64.to_le_bytes())?; // hd_start_distance_m (8 bytes)

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write FH block (File History) - Mandatory for MDF 4.x
    ///
    /// FH block structure:
    /// - Header: 24 bytes (##FH + reserved + length + link_count)
    /// - Links (6): fh_fh_next, fh_md_comment, fh_tx_tool_id, fh_tx_tool_vendor, fh_tx_tool_version, fh_tx_user_name
    /// - Data: fh_time_ns (8), fh_tz_offset (2), fh_dst_offset (2), fh_flags (1), fh_reserved (1)
    /// - Total: 24 + 48 + 14 = 86 bytes, padded to 88 for alignment
    pub fn write_fh_block(&mut self, fh: &FhBlock) -> WriteResult<u64> {
        // Write TX blocks for strings first
        let tx_tool_id = self.write_tx_block(&TxBlock::new(&fh.fh_tool_id))?;
        let tx_tool_vendor = self.write_tx_block(&TxBlock::new(&fh.fh_tool_vendor))?;
        let tx_tool_version = self.write_tx_block(&TxBlock::new(&fh.fh_tool_version))?;
        let tx_user_name = self.write_tx_block(&TxBlock::new(&fh.fh_user_name))?;

        // Re-align after TX blocks
        let fh_offset = self.align_to_8()?;

        // Block ID
        self.writer.write_all(block_id::FH)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length (8 bytes) - 88 bytes (86 + 2 padding)
        self.writer.write_all(&88u64.to_le_bytes())?;
        // Link count (6 links)
        self.writer.write_all(&6u64.to_le_bytes())?;

        // Links
        self.writer.write_all(&fh.fh_fh_next.to_le_bytes())?;
        self.writer.write_all(&fh.fh_md_comment.to_le_bytes())?;
        self.writer.write_all(&tx_tool_id.to_le_bytes())?;
        self.writer.write_all(&tx_tool_vendor.to_le_bytes())?;
        self.writer.write_all(&tx_tool_version.to_le_bytes())?;
        self.writer.write_all(&tx_user_name.to_le_bytes())?;

        // Data fields
        self.writer.write_all(&fh.fh_time_ns.to_le_bytes())?;
        self.writer.write_all(&fh.fh_tz_offset.to_le_bytes())?;
        self.writer.write_all(&fh.fh_dst_offset.to_le_bytes())?;
        self.writer.write_all(&[0u8; 2])?; // fh_flags (1) + fh_reserved (1)
        // Padding to 88 bytes (2 more bytes)
        self.writer.write_all(&[0u8; 2])?;

        self.current_offset = self.writer.stream_position()?;
        Ok(fh_offset)
    }

    /// Write DG block
    pub fn write_dg_block(&mut self, dg: &DgBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // Block ID
        self.writer.write_all(block_id::DG)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length (8 bytes) - 64 bytes
        self.writer.write_all(&64u64.to_le_bytes())?;
        // Link count (4 links)
        self.writer.write_all(&4u64.to_le_bytes())?;
        // Links
        self.writer.write_all(&dg.dg_dg_next.to_le_bytes())?;
        self.writer.write_all(&dg.dg_cg_first.to_le_bytes())?;
        self.writer.write_all(&dg.dg_data.to_le_bytes())?;
        self.writer.write_all(&dg.dg_md_comment.to_le_bytes())?;
        // Data
        self.writer.write_all(&dg.dg_rec_id_size.to_le_bytes())?;
        // Reserved (7 bytes)
        self.writer.write_all(&[0u8; 7])?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write CG block
    ///
    /// CG block structure:
    /// - Links (6): cg_cg_next, cg_cn_first, cg_tx_acq_name, cg_si_acq_source, cg_cg_master, cg_md_comment
    /// - Data: cg_record_id (8), cg_cycle_count (8), cg_flags (2), cg_path_separator (2), cg_reserved (4), cg_data_bytes (4), cg_inval_bytes (4)
    /// - Total: 24 + 48 + 32 = 104 bytes
    pub fn write_cg_block(&mut self, cg: &CgBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // Block ID
        self.writer.write_all(block_id::CG)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length (8 bytes) - 104 bytes
        self.writer.write_all(&104u64.to_le_bytes())?;
        // Link count (6 links)
        self.writer.write_all(&6u64.to_le_bytes())?;

        // Links (6)
        self.writer.write_all(&cg.cg_cg_next.to_le_bytes())?;
        self.writer.write_all(&cg.cg_cn_first.to_le_bytes())?;
        self.writer.write_all(&cg.cg_tx_acq_name.to_le_bytes())?;
        self.writer.write_all(&cg.cg_si_acq_source.to_le_bytes())?;
        self.writer.write_all(&0u64.to_le_bytes())?;    // cg_cg_master (optional, 0 if not used)
        self.writer.write_all(&cg.cg_md_comment.to_le_bytes())?;

        // Data fields (in correct order)
        self.writer.write_all(&cg.cg_record_id.to_le_bytes())?;   // cg_record_id (8 bytes)
        self.writer.write_all(&cg.cg_cycle_count.to_le_bytes())?; // cg_cycle_count (8 bytes)
        self.writer.write_all(&cg.cg_flags.to_le_bytes())?;       // cg_flags (2 bytes)
        self.writer.write_all(&cg.cg_path_separator.to_le_bytes())?; // cg_path_separator (2 bytes)
        self.writer.write_all(&[0u8; 4])?;                         // cg_reserved (4 bytes)
        self.writer.write_all(&cg.cg_data_bytes.to_le_bytes())?; // cg_data_bytes (4 bytes)
        self.writer.write_all(&cg.cg_inval_bytes.to_le_bytes())?; // cg_inval_bytes (4 bytes)

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write CN block
    pub fn write_cn_block(&mut self, cn: &CnBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // Block ID
        self.writer.write_all(block_id::CN)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length (8 bytes) - 160 bytes
        self.writer.write_all(&160u64.to_le_bytes())?;
        // Link count (8)
        self.writer.write_all(&8u64.to_le_bytes())?;

        // Links (8)
        self.writer.write_all(&cn.cn_cn_next.to_le_bytes())?;
        self.writer.write_all(&cn.cn_composition.to_le_bytes())?;
        self.writer.write_all(&cn.cn_tx_name.to_le_bytes())?;
        self.writer.write_all(&cn.cn_si_source.to_le_bytes())?;
        self.writer.write_all(&cn.cn_cc_conversion.to_le_bytes())?;
        self.writer.write_all(&cn.cn_data.to_le_bytes())?;
        self.writer.write_all(&cn.cn_md_unit.to_le_bytes())?;
        self.writer.write_all(&cn.cn_md_comment.to_le_bytes())?;

        // Data fields (in correct order per cn.toml)
        self.writer.write_all(&[cn.cn_type])?;           // cn_type (1 byte)
        self.writer.write_all(&[cn.cn_sync_type])?;      // cn_sync_type (1 byte)
        self.writer.write_all(&[cn.cn_data_type])?;      // cn_data_type (1 byte)
        self.writer.write_all(&[cn.cn_bit_offset])?;     // cn_bit_offset (1 byte)
        self.writer.write_all(&cn.cn_byte_offset.to_le_bytes())?; // cn_byte_offset (4 bytes)
        self.writer.write_all(&cn.cn_bit_count.to_le_bytes())?; // cn_bit_count (4 bytes)
        self.writer.write_all(&cn.cn_flags.to_le_bytes())?; // cn_flags (4 bytes)
        self.writer.write_all(&cn.cn_inval_bit_pos.to_le_bytes())?; // cn_inval_bit_pos (4 bytes)
        self.writer.write_all(&[cn.cn_precision])?;      // cn_precision (1 byte)
        self.writer.write_all(&[0u8])?;                   // cn_reserved (1 byte)
        self.writer.write_all(&cn.cn_attachment_count.to_le_bytes())?; // cn_attachment_count (2 bytes)
        self.writer.write_all(&cn.cn_val_limit_1.to_le_bytes())?; // cn_val_range_min (8 bytes)
        self.writer.write_all(&cn.cn_val_limit_2.to_le_bytes())?; // cn_val_range_max (8 bytes)
        self.writer.write_all(&0f64.to_le_bytes())?;     // cn_limit_min (8 bytes)
        self.writer.write_all(&0f64.to_le_bytes())?;     // cn_limit_max (8 bytes)
        self.writer.write_all(&0f64.to_le_bytes())?;     // cn_limit_ext_min (8 bytes)
        self.writer.write_all(&0f64.to_le_bytes())?;     // cn_limit_ext_max (8 bytes)

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write TX block
    pub fn write_tx_block(&mut self, tx: &TxBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        let text_bytes = tx.tx_data.as_bytes();
        let text_len = text_bytes.len() + 1; // Include null terminator

        // block_len must reflect the actual bytes on disk including padding,
        // because MDF4 readers use bl_len to navigate to the next block.
        let remainder = text_len % 8;
        let padding = if remainder != 0 { 8 - remainder } else { 0 };
        let block_len = 24 + text_len + padding;

        // Block ID
        self.writer.write_all(block_id::TX)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length (includes padding)
        self.writer.write_all(&(block_len as u64).to_le_bytes())?;
        // Link count (0)
        self.writer.write_all(&0u64.to_le_bytes())?;
        // Text data (null-terminated)
        self.writer.write_all(text_bytes)?;
        self.writer.write_all(&[0u8; 1])?; // Null terminator

        // Pad to 8-byte alignment
        if padding != 0 {
            self.writer.write_all(&vec![0u8; padding])?;
        }

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write DT block (uncompressed data)
    pub fn write_dt_block(&mut self, dt: &DtBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        let block_len = 24 + dt.data.len() as u64;

        // Block ID
        self.writer.write_all(block_id::DT)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length
        self.writer.write_all(&block_len.to_le_bytes())?;
        // Link count (0)
        self.writer.write_all(&0u64.to_le_bytes())?;
        // Data
        self.writer.write_all(&dt.data)?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write DZ block (compressed data)
    ///
    /// DZ block contains compressed data using deflate or transpose+deflate.
    ///
    /// # Arguments
    /// * `dz` - DZ block containing compressed data and metadata
    ///
    /// # Example
    /// ```ignore
    /// use mf4_parse::writer::compression::Compressor;
    ///
    /// let compressor = Compressor::deflate();
    /// let (compressed, original_len) = compressor.compress(&data)?;
    ///
    /// let dz = DzBlock {
    ///     dz_org_data_length: original_len,
    ///     dz_data_length: compressed.len() as u64,
    ///     dz_zip_type: 0, // Deflate
    ///     dz_zip_parameter: 0,
    ///     data: compressed,
    /// };
    /// let dz_offset = writer.write_dz_block(&dz)?;
    /// ```
    pub fn write_dz_block(&mut self, dz: &DzBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // DZ block structure per MDF spec:
        // Header: 24 bytes (id + reserved + length + link_count)
        // Links: 0 (DZ has no links per MDF4 spec)
        // Data fields:
        //   dz_org_block_type: 2 bytes (CHAR) - "DT" for data block
        //   dz_zip_type: 1 byte (UINT8) - 0=Deflate, 1=Transpose+Deflate
        //   dz_reserved: 1 byte (BYTE)
        //   dz_zip_parameter: 4 bytes (UINT32) - column count for transpose
        //   dz_org_data_length: 8 bytes (UINT64) - original uncompressed size
        //   dz_data_length: 8 bytes (UINT64) - compressed size
        // Total header: 24 + 0 + 2 + 1 + 1 + 4 + 8 + 8 = 48 bytes, plus data

        let block_len = 48u64 + dz.data.len() as u64;

        // Block ID
        self.writer.write_all(block_id::DZ)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length
        self.writer.write_all(&block_len.to_le_bytes())?;
        // Link count (0 - DZ has no links)
        self.writer.write_all(&0u64.to_le_bytes())?;

        // Data fields (in MDF spec order)
        // dz_org_block_type: 2 bytes CHAR - "DT" indicates compressed DT block
        self.writer.write_all(b"DT")?;
        // dz_zip_type: 1 byte UINT8
        self.writer.write_all(&dz.dz_zip_type.to_le_bytes())?;
        // dz_reserved: 1 byte
        self.writer.write_all(&[0u8; 1])?;
        // dz_zip_parameter: 4 bytes UINT32
        self.writer.write_all(&dz.dz_zip_parameter.to_le_bytes())?;
        // dz_org_data_length: 8 bytes UINT64
        self.writer.write_all(&dz.dz_org_data_length.to_le_bytes())?;
        // dz_data_length: 8 bytes UINT64
        self.writer.write_all(&dz.dz_data_length.to_le_bytes())?;

        // Compressed data
        self.writer.write_all(&dz.data)?;

        // Align to 8 bytes
        self.align_to_8()?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write DL block (data link for chaining multiple data blocks)
    ///
    /// DL block links multiple DT/DZ blocks together, allowing streaming writes
    /// to extend data without rewriting the entire file.
    ///
    /// # Arguments
    /// * `links` - List of file offsets pointing to DT/DZ blocks
    ///
    /// # Example
    /// ```ignore
    /// let links = vec![dt1_offset, dt2_offset, dt3_offset];
    /// let offsets = vec![0, 1024, 2048]; // cumulative byte offsets
    /// let dl_offset = writer.write_dl_block(&links, &offsets)?;
    /// // Now DG.dg_data should point to dl_offset
    /// ```
    /// Write a DL block with per-block byte offsets.
    ///
    /// `links` - file offsets to DT/DZ blocks
    /// `data_offsets` - cumulative byte offset within the uncompressed data for each block
    pub fn write_dl_block(&mut self, links: &[u64], data_offsets: &[u64]) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        let link_count = links.len() as u64 + 1; // +1 for dl_dl_next
        let dl_count = links.len() as u32;
        // data section: flags(1) + reserved(3) + count(4) + offsets(dl_count * 8)
        let data_section_len = 1 + 3 + 4 + (dl_count as u64) * 8;
        let block_len = 24 + link_count * 8 + data_section_len;

        // Block ID
        self.writer.write_all(block_id::DL)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length
        self.writer.write_all(&block_len.to_le_bytes())?;
        // Link count
        self.writer.write_all(&link_count.to_le_bytes())?;

        // dl_dl_next (0 = no more DL blocks in chain)
        self.writer.write_all(&0u64.to_le_bytes())?;

        // Links to data blocks
        for link in links {
            self.writer.write_all(&link.to_le_bytes())?;
        }

        // dl_flags (0 = no equal length, no time/angle/distance values)
        self.writer.write_all(&[0u8; 1])?;
        // Reserved (3 bytes)
        self.writer.write_all(&[0u8; 3])?;
        // dl_count (number of data blocks)
        self.writer.write_all(&dl_count.to_le_bytes())?;
        // dl_offset (byte offsets within concatenated uncompressed data)
        for off in data_offsets {
            self.writer.write_all(&off.to_le_bytes())?;
        }

        // Align to 8 bytes
        self.align_to_8()?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write HL block (Header List for compressed DL chains)
    ///
    /// HL block is required when DZ blocks are linked via DL. The hierarchy is:
    /// DG.dg_data → HL → DL → DZ blocks
    ///
    /// # Arguments
    /// * `hl` - HL block data containing the link to the first DL block
    pub fn write_hl_block(&mut self, hl: &HlBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // HL block layout:
        // Header: 24 bytes (id[4] + reserved[4] + length[8] + link_count[8])
        // Links: 1 × 8 = 8 bytes (hl_dl_first)
        // Data: hl_flags(2) + hl_zip_type(1) + reserved(5) = 8 bytes
        // Total: 24 + 8 + 8 = 40 bytes
        let block_len = 40u64;

        // Block ID
        self.writer.write_all(block_id::HL)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length
        self.writer.write_all(&block_len.to_le_bytes())?;
        // Link count (1 - hl_dl_first)
        self.writer.write_all(&1u64.to_le_bytes())?;

        // hl_dl_first link
        self.writer.write_all(&hl.hl_dl_first.to_le_bytes())?;

        // hl_flags: 2 bytes UINT16
        self.writer.write_all(&hl.hl_flags.to_le_bytes())?;
        // hl_zip_type: 1 byte UINT8
        self.writer.write_all(&hl.hl_zip_type.to_le_bytes())?;
        // hl_reserved: 5 bytes
        self.writer.write_all(&[0u8; 5])?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write CC block (conversion rules)
    ///
    /// CC block defines how raw data values are converted to physical values.
    /// Common conversion types:
    /// - 0: 1:1 (no conversion)
    /// - 1: Linear (y = p1 + p2 * x)
    /// - 4/5: Table lookup (with/without interpolation)
    pub fn write_cc_block(&mut self, cc: &CcBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        let link_count = 4 + cc.cc_ref.len() as u64;
        let val_bytes = (cc.cc_val.len() * 8) as u64;
        let block_len = 24 + link_count * 8 + 24 + val_bytes;

        // Block ID
        self.writer.write_all(block_id::CC)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length
        self.writer.write_all(&block_len.to_le_bytes())?;
        // Link count
        self.writer.write_all(&link_count.to_le_bytes())?;

        // Links
        self.writer.write_all(&cc.cc_tx_name.to_le_bytes())?;
        self.writer.write_all(&cc.cc_md_comment.to_le_bytes())?;
        self.writer.write_all(&cc.cc_cc_inverse.to_le_bytes())?;
        self.writer.write_all(&cc.cc_tx_unit.to_le_bytes())?;

        // Ref links
        for link in &cc.cc_ref {
            self.writer.write_all(&link.to_le_bytes())?;
        }

        // Data fields
        self.writer.write_all(&cc.cc_type.to_le_bytes())?;
        self.writer.write_all(&cc.cc_precision.to_le_bytes())?;
        self.writer.write_all(&cc.cc_flags.to_le_bytes())?;
        self.writer.write_all(&cc.cc_ref_count.to_le_bytes())?;
        self.writer.write_all(&cc.cc_val_count.to_le_bytes())?;

        // Physical range
        self.writer.write_all(&cc.cc_phy_range_min.to_le_bytes())?;
        self.writer.write_all(&cc.cc_phy_range_max.to_le_bytes())?;

        // cc_val values
        for val in &cc.cc_val {
            self.writer.write_all(&val.to_le_bytes())?;
        }

        // Align to 8 bytes
        self.align_to_8()?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Write SI block (source information)
    ///
    /// SI block contains source information for a channel or channel group.
    /// Used to identify where the data originated (ECU, bus, tool, etc.)
    pub fn write_si_block(&mut self, si: &SiBlock) -> WriteResult<u64> {
        let offset = self.align_to_8()?;

        // Block ID
        self.writer.write_all(block_id::SI)?;
        // Reserved (4 bytes)
        self.writer.write_all(&[0u8; 4])?;
        // Block length (56 bytes)
        self.writer.write_all(&56u64.to_le_bytes())?;
        // Link count (3)
        self.writer.write_all(&3u64.to_le_bytes())?;

        // Links
        self.writer.write_all(&si.si_tx_name.to_le_bytes())?;
        self.writer.write_all(&si.si_tx_path.to_le_bytes())?;
        self.writer.write_all(&si.si_md_comment.to_le_bytes())?;

        // Data fields
        self.writer.write_all(&si.si_type.to_le_bytes())?;
        self.writer.write_all(&si.si_bus_type.to_le_bytes())?;
        self.writer.write_all(&si.si_flags.to_le_bytes())?;
        // Reserved (5 bytes)
        self.writer.write_all(&[0u8; 5])?;

        // Align to 8 bytes
        self.align_to_8()?;

        self.current_offset = self.writer.stream_position()?;
        Ok(offset)
    }

    /// Update a link at a specific offset
    pub fn update_link(&mut self, offset: u64, new_link: u64) -> WriteResult<()> {
        let current_pos = self.current_offset;
        self.seek(offset)?;
        self.writer.write_all(&new_link.to_le_bytes())?;
        self.seek(current_pos)?;
        Ok(())
    }

    /// Write raw bytes
    pub fn write_bytes(&mut self, data: &[u8]) -> WriteResult<()> {
        self.writer.write_all(data)?;
        self.current_offset += data.len() as u64;
        Ok(())
    }
}
