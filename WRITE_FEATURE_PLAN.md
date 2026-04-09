# MF4 Write Feature Implementation Plan

## Overview

This document outlines the implementation plan for adding write capabilities to the `mf4_parse` crate. Currently, the library only supports reading MF4 files. This feature will enable:

1. **One-time Write Mode**: Create and write complete MF4 files in a single operation
2. **Streaming Write Mode**: Create MF4 files and incrementally append data over time
3. **Data Compression**: Support for compressed data blocks (DZ blocks) for large datasets

---

## Part 1: Feature List

### 1.1 Core Writing Features

| Feature | Description | Priority |
|---------|-------------|----------|
| MF4 File Creation | Create new MF4 files with proper header structure | P0 |
| ID/HD Block Write | Write file identification and header blocks | P0 |
| DG Block Write | Write Data Group blocks | P0 |
| CG Block Write | Write Channel Group blocks | P0 |
| CN Block Write | Write Channel blocks | P0 |
| DT Block Write | Write uncompressed data blocks | P0 |
| One-time Write | Write complete file in single operation | P0 |
| Streaming Write | Incremental data append capability | P1 |
| DZ Block Write | Write compressed data blocks (deflate) | P1 |
| DL/HL Block Write | Write data link and hierarchy blocks | P2 |
| TX/MD Block Write | Write text/metadata blocks | P0 |
| CC Block Write | Write conversion blocks | P1 |
| SI Block Write | Write source information blocks | P1 |
| CA Block Write | Write channel array blocks | P2 |

### 1.2 Data Type Support

| Data Type | MDF Type Code | Priority |
|-----------|---------------|----------|
| UINT8/UINT16/UINT32/UINT64 | 0 | P0 |
| INT8/INT16/INT32/INT64 | 2,3 | P0 |
| FLOAT32/FLOAT64 | 4,5 | P0 |
| String (UTF-8) | 6,7 | P1 |
| String (UTF-16) | 8,9 | P2 |
| Byte Array | 10 | P2 |

### 1.3 Compression Support

| Compression Type | Code | Priority |
|------------------|------|----------|
| Deflate | 0 | P1 |
| Transpose + Deflate | 1 | P2 |

---

## Part 2: Architecture Design

### 2.1 Module Structure

```
src/
├── writer/
│   ├── mod.rs              # Writer module exports
│   ├── builder.rs          # Mf4Builder - high-level API
│   ├── stream_writer.rs    # Mf4StreamWriter - streaming API
│   ├── block_writer.rs     # Low-level block writing primitives
│   ├── buffer.rs           # Write buffer management
│   └── compression.rs      # DZ block compression utilities
├── components_write/
│   ├── mod.rs
│   ├── dg_write.rs         # DataGroup write structures
│   ├── cg_write.rs         # ChannelGroup write structures
│   ├── cn_write.rs         # Channel write structures
│   └── cc_write.rs         # Conversion write structures
└── config_write/
    └── *.toml              # Block write templates (reuse existing)
```

### 2.2 Core Data Structures

#### 2.2.1 Mf4Builder (One-time Write)

```rust
/// Builder for creating MF4 files in a single write operation
pub struct Mf4Builder {
    /// File metadata
    metadata: Mf4Metadata,
    /// Collection of data groups to write
    data_groups: Vec<DataGroupBuilder>,
    /// Compression settings
    compression: Option<CompressionConfig>,
}

/// Metadata for MF4 file
pub struct Mf4Metadata {
    /// File version (4.10, 4.11, etc.)
    pub version: String,
    /// Timestamp (nanoseconds since 1970-01-01)
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

/// Compression configuration
pub struct CompressionConfig {
    /// Compression type (0=Deflate, 1=Transpose+Deflate)
    pub zip_type: u8,
    /// Minimum data size to trigger compression (bytes)
    pub min_size: u64,
    /// Compression level (1-9)
    pub level: u8,
}

/// Builder for a single data group
pub struct DataGroupBuilder {
    /// Channel groups within this data group
    channel_groups: Vec<ChannelGroupBuilder>,
    /// Record ID size (0, 1, 2, 4, 8 bytes)
    rec_id_size: u8,
    /// Comment
    comment: Option<String>,
}

/// Builder for a channel group
pub struct ChannelGroupBuilder {
    /// Acquisition name
    acq_name: String,
    /// Record ID (for multiple channel groups)
    record_id: u64,
    /// Channels in this group
    channels: Vec<ChannelBuilder>,
    /// Master channel (time) configuration
    master: Option<ChannelBuilder>,
    /// Channel group flags
    flags: u16,
}

/// Builder for a channel
pub struct ChannelBuilder {
    /// Channel name
    name: String,
    /// Data type (0-10)
    data_type: u8,
    /// Unit string
    unit: Option<String>,
    /// Comment
    comment: Option<String>,
    /// Sync type (0=None, 1=Time, 2=Angle, 3=Distance, 4=Index)
    sync_type: u8,
    /// Channel type (0=Fixed, 1=VLSD, 2=Master, 3=VirtualMaster)
    cn_type: u8,
    /// Conversion configuration
    conversion: Option<ConversionBuilder>,
    /// Array dimensions (for array channels)
    array_dims: Option<Vec<u32>>,
}

/// Builder for conversion rules
pub struct ConversionBuilder {
    /// Conversion type (0=1:1, 1=Linear, etc.)
    cc_type: u8,
    /// Conversion parameters
    params: ConversionParams,
    /// Unit string
    unit: Option<String>,
}

pub enum ConversionParams {
    OneToOne,
    Linear { p1: f64, p2: f64 },  // y = p1 + p2 * x
    Rational { coeffs: [f64; 6] },
    Table { keys: Vec<f64>, values: Vec<f64>, interpolate: bool },
    Value2Text { keys: Vec<f64>, texts: Vec<String> },
}
```

#### 2.2.2 Mf4StreamWriter (Streaming Write)

```rust
/// Streaming writer for incremental data append
pub struct Mf4StreamWriter {
    /// File handle
    file: BufWriter<File>,
    /// File path
    path: PathBuf,
    /// Metadata
    metadata: Mf4Metadata,
    /// Data groups with streaming capability
    data_groups: Vec<StreamingDataGroup>,
    /// Write buffer for performance
    buffer: Vec<u8>,
    /// Buffer flush threshold
    flush_threshold: usize,
    /// File state tracking
    state: WriterState,
}

/// Streaming data group that supports incremental writes
pub struct StreamingDataGroup {
    /// Channel group definition (fixed after creation)
    channel_groups: Vec<ChannelGroupDef>,
    /// Record ID size
    rec_id_size: u8,
    /// Current cycle count
    cycle_count: u64,
    /// Data block offset in file
    data_offset: u64,
    /// Data buffer (accumulated records)
    data_buffer: Vec<u8>,
    /// Compression settings
    compression: Option<CompressionConfig>,
    /// Pending records to write
    pending_records: Vec<RecordData>,
}

/// Definition of a channel group (immutable after creation)
pub struct ChannelGroupDef {
    /// Channel definitions
    channels: Vec<ChannelDef>,
    /// Master channel definition
    master: Option<ChannelDef>,
    /// Record ID
    record_id: u64,
    /// Record size in bytes
    record_size: u32,
}

/// Definition of a channel (immutable after creation)
pub struct ChannelDef {
    name: String,
    data_type: u8,
    byte_offset: u32,
    bit_offset: u8,
    bit_count: u32,
    unit: Option<String>,
}

/// Record data for streaming write
pub struct RecordData {
    /// Record ID (for multi-CG scenarios)
    record_id: u64,
    /// Raw record bytes
    data: Vec<u8>,
}

/// Writer state machine
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
```

#### 2.2.3 BlockWriter (Low-level Primitives)

```rust
/// Low-level block writer
pub struct BlockWriter<'a, W: Write + Seek> {
    writer: &'a mut W,
    current_offset: u64,
}

impl<'a, W: Write + Seek> BlockWriter<'a, W> {
    /// Write ID block (file identification)
    pub fn write_id_block(&mut self, version: &str) -> Result<u64, WriteError>;
    
    /// Write HD block (header)
    pub fn write_hd_block(&mut self, header: &HeaderBlock) -> Result<u64, WriteError>;
    
    /// Write DG block
    pub fn write_dg_block(&mut self, dg: &DGBlock) -> Result<u64, WriteError>;
    
    /// Write CG block
    pub fn write_cg_block(&mut self, cg: &CGBlock) -> Result<u64, WriteError>;
    
    /// Write CN block
    pub fn write_cn_block(&mut self, cn: &CNBlock) -> Result<u64, WriteError>;
    
    /// Write DT block (uncompressed data)
    pub fn write_dt_block(&mut self, data: &[u8]) -> Result<u64, WriteError>;
    
    /// Write DZ block (compressed data)
    pub fn write_dz_block(&mut self, data: &[u8], zip_type: u8) -> Result<u64, WriteError>;
    
    /// Write TX block (text)
    pub fn write_tx_block(&mut self, text: &str) -> Result<u64, WriteError>;
    
    /// Write DL block (data link for multiple data blocks)
    pub fn write_dl_block(&mut self, links: &[u64]) -> Result<u64, WriteError>;
    
    /// Update link at specific offset
    pub fn update_link(&mut self, offset: u64, new_link: u64) -> Result<(), WriteError>;
    
    /// Get current file position
    pub fn position(&self) -> u64;
}
```

---

## Part 3: API Design with Examples

### 3.1 One-time Write API (Mf4Builder)

#### Example 1: Simple Time Series Write

```rust
use mf4_parse::writer::{Mf4Builder, Mf4Metadata, DataGroupBuilder, ChannelGroupBuilder, ChannelBuilder};
use std::path::PathBuf;

fn example_simple_write() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create metadata
    let metadata = Mf4Metadata {
        version: "4.10".to_string(),
        start_time_ns: 1704067200000000000, // 2024-01-01 00:00:00
        author: Some("Test User".to_string()),
        organization: Some("Test Org".to_string()),
        project: Some("Test Project".to_string()),
        comment: Some("Simple test measurement".to_string()),
    };
    
    // 2. Create builder
    let mut builder = Mf4Builder::new(metadata);
    
    // 3. Define channels
    let time_channel = ChannelBuilder::new_master_time("time");
    let temp_channel = ChannelBuilder::new("Temperature")
        .data_type(4)      // FLOAT32
        .unit("°C")
        .comment("Engine temperature")
        .build();
    let rpm_channel = ChannelBuilder::new("RPM")
        .data_type(2)      // UINT16
        .unit("rpm")
        .build();
    
    // 4. Create channel group
    let cg = ChannelGroupBuilder::new()
        .name("EngineData")
        .master(time_channel)
        .channel(temp_channel)
        .channel(rpm_channel)
        .build()?;
    
    // 5. Create data group
    let dg = DataGroupBuilder::new()
        .channel_group(cg)
        .build();
    
    builder.add_data_group(dg);
    
    // 6. Add data (time, temperature, rpm)
    let time_data: Vec<f64> = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let temp_data: Vec<f32> = vec![20.0, 21.5, 23.0, 24.5, 26.0];
    let rpm_data: Vec<u16> = vec![1000, 1500, 2000, 2500, 3000];
    
    builder.set_channel_data("time", &time_data)?;
    builder.set_channel_data("Temperature", &temp_data)?;
    builder.set_channel_data("RPM", &rpm_data)?;
    
    // 7. Write to file
    builder.write(PathBuf::from("output.mf4"))?;
    
    Ok(())
}
```

#### Example 2: With Compression

```rust
use mf4_parse::writer::{Mf4Builder, CompressionConfig};

fn example_compressed_write() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = Mf4Builder::new(Mf4Metadata::default());
    
    // Enable compression for data > 1MB
    builder.set_compression(CompressionConfig {
        zip_type: 0,        // Deflate
        min_size: 1_000_000,
        level: 6,           // Default compression level
    });
    
    // ... add channels and data ...
    
    builder.write(PathBuf::from("compressed.mf4"))?;
    Ok(())
}
```

#### Example 3: With Conversion

```rust
use mf4_parse::writer::{ChannelBuilder, ConversionBuilder, ConversionParams};

fn example_with_conversion() -> Result<(), Box<dyn std::error::Error>> {
    // Channel with linear conversion (raw -> physical)
    // Physical = 10.0 + 0.5 * Raw
    let conversion = ConversionBuilder::linear(10.0, 0.5)
        .unit("V");
    
    let channel = ChannelBuilder::new("Voltage")
        .data_type(0)       // UINT8 raw type
        .conversion(conversion)
        .build();
    
    // Raw data will be converted on read
    let raw_data: Vec<u8> = vec![0, 50, 100, 150, 200];
    
    // ... build and write ...
    Ok(())
}
```

### 3.2 Streaming Write API (Mf4StreamWriter)

#### Example 4: Basic Streaming Write

```rust
use mf4_parse::writer::{Mf4StreamWriter, Mf4Metadata, StreamingDataGroup, ChannelGroupDef, ChannelDef};
use std::path::PathBuf;

fn example_streaming_write() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create streaming writer
    let mut writer = Mf4StreamWriter::new(
        PathBuf::from("streaming.mf4"),
        Mf4Metadata::default()
    )?;
    
    // 2. Define channel structure (fixed after creation)
    let time_def = ChannelDef::new_master("time");
    let temp_def = ChannelDef::new("Temperature")
        .data_type(5)  // FLOAT64
        .unit("°C");
    
    let cg_def = ChannelGroupDef::new()
        .name("Measurement")
        .master(time_def)
        .channel(temp_def)
        .build()?;
    
    // 3. Create data group with channel definition
    let dg = StreamingDataGroup::new(cg_def)?;
    writer.add_data_group(dg)?;
    
    // 4. Finalize structure (ready for data)
    writer.finalize_structure()?;
    
    // 5. Write data incrementally (simulating real-time acquisition)
    for i in 0..100 {
        let time = i as f64 * 0.01;
        let temp = 20.0 + (i as f64 * 0.1).sin();
        
        // Append one record
        writer.append_record("time", &time)?;
        writer.append_record("Temperature", &temp)?;
        writer.flush_record()?;  // Complete current record
        
        // Optionally flush to disk periodically
        if i % 10 == 0 {
            writer.flush()?;
        }
    }
    
    // 6. Finalize and close file
    writer.finalize()?;
    
    Ok(())
}
```

#### Example 5: Multi-Channel Group Streaming

```rust
fn example_multi_cg_streaming() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = Mf4StreamWriter::new(
        PathBuf::from("multi_cg.mf4"),
        Mf4Metadata::default()
    )?;
    
    // Define two channel groups with different record IDs
    let cg1 = ChannelGroupDef::new()
        .name("GroupA")
        .record_id(1)
        .master(ChannelDef::new_master("time_a"))
        .channel(ChannelDef::new("SignalA").data_type(5))
        .build()?;
    
    let cg2 = ChannelGroupDef::new()
        .name("GroupB")
        .record_id(2)
        .master(ChannelDef::new_master("time_b"))
        .channel(ChannelDef::new("SignalB").data_type(5))
        .build()?;
    
    // Add data groups
    let dg = StreamingDataGroup::new()
        .channel_group(cg1)
        .channel_group(cg2)
        .rec_id_size(1)  // 1-byte record ID
        .build()?;
    
    writer.add_data_group(dg)?;
    writer.finalize_structure()?;
    
    // Write interleaved records from different groups
    for i in 0..100 {
        // Write to Group A
        writer.append_record_by_id(1, "time_a", &(i as f64 * 0.01))?;
        writer.append_record_by_id(1, "SignalA", &(i as f64 * 2.0))?;
        writer.flush_record_by_id(1)?;
        
        // Write to Group B (different rate)
        if i % 2 == 0 {
            writer.append_record_by_id(2, "time_b", &(i as f64 * 0.02))?;
            writer.append_record_by_id(2, "SignalB", &(i as f64 * 3.0))?;
            writer.flush_record_by_id(2)?;
        }
    }
    
    writer.finalize()?;
    Ok(())
}
```

### 3.3 Read-Modify-Write Pattern

#### Example 6: Modify Existing File

```rust
use mf4_parse::{Mf4Wrapper, Mf4Metadata};
use mf4_parse::writer::Mf4Builder;

fn example_modify_existing() -> Result<(), Box<dyn std::error::Error>> {
    // Read existing file
    let original = Mf4Wrapper::new(PathBuf::from("input.mf4"), None)?;
    
    // Convert to builder for modification
    let mut builder = Mf4Builder::from_reader(&original)?;
    
    // Add new channel
    let new_channel = ChannelBuilder::new("DerivedSignal")
        .data_type(5)
        .unit("m/s")
        .build();
    
    builder.add_channel_to_group("EngineData", new_channel)?;
    
    // Compute derived data
    let rpm: Vec<f64> = original.get_channel_data("RPM")?.try_into()?;
    let derived: Vec<f64> = rpm.iter().map(|r| r * 0.1).collect();
    
    builder.set_channel_data("DerivedSignal", &derived)?;
    
    // Write modified file
    builder.write(PathBuf::from("modified.mf4"))?;
    Ok(())
}
```

---

## Part 4: Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)

1. **BlockWriter Implementation**
   - Implement `BlockWriter` trait with basic block writing
   - Create write error types
   - Implement block header writing utilities

2. **TX/MD Block Write**
   - Text block serialization
   - Metadata block serialization

3. **ID/HD Block Write**
   - File identification block
   - Header block with metadata

### Phase 2: Structure Blocks (Week 3-4)

1. **CN Block Write**
   - Channel definition serialization
   - Support for all data types
   - Bit offset/byte offset calculation

2. **CG Block Write**
   - Channel group serialization
   - Record size calculation
   - Channel offset assignment

3. **DG Block Write**
   - Data group serialization
   - Link management between blocks

### Phase 3: Data Blocks (Week 5-6)

1. **DT Block Write**
   - Raw data serialization
   - Record packing

2. **DZ Block Write**
   - Compression implementation
   - Deflate integration
   - Transpose algorithm (optional)

3. **DL/HL Block Write**
   - Data link block for multiple data segments
   - Hierarchy block support

### Phase 4: High-Level APIs (Week 7-8)

1. **Mf4Builder**
   - Builder pattern implementation
   - Data validation
   - File finalization

2. **Mf4StreamWriter**
   - Streaming architecture
   - Buffer management
   - Incremental updates

3. **CC/SI Block Write**
   - Conversion block serialization
   - Source information serialization

### Phase 5: Testing & Documentation (Week 9-10)

1. **Unit Tests**
   - Block-level tests
   - Round-trip tests (write then read)

2. **Integration Tests**
   - Large file tests
   - Multi-DG/CG tests
   - Compression tests

3. **Documentation**
   - API documentation
   - Examples
   - Migration guide

---

## Part 5: Technical Considerations

### 5.1 Block Link Management

MF4 files use a linked-list structure where each block contains offsets to related blocks. Write operations must:

1. **Pre-calculate block sizes** - Know final positions before writing
2. **Update links in reverse order** - Write blocks first, then update parent links
3. **Handle 8-byte alignment** - All blocks must start at 8-byte aligned offsets

**Strategy: Two-pass writing**
- Pass 1: Calculate all block sizes and relative offsets
- Pass 2: Write blocks with final absolute offsets

### 5.2 Streaming Write Challenges

For streaming mode, the key challenge is updating the file structure without rewriting:

1. **CG cycle_count update** - Must be updated as records are added
2. **DT block growth** - Either append to existing DT or create DL-linked chain
3. **File seek overhead** - Minimize seeks by buffering

**Strategy: Write-ahead structure**
1. Write ID/HD/DG/CG/CN blocks with placeholders
2. Write data blocks incrementally
3. Update counters/pointers on flush/finalize

### 5.3 Compression Considerations

DZ blocks require:
- Original data length storage
- Compression type flag
- Optional transposition for columnar data

**When to compress:**
- Data size > threshold (configurable)
- Sequential write: compress on finalize
- Streaming write: compress in segments

### 5.4 Memory Management

For large files:
- Avoid loading all data into memory
- Use streaming writes for large datasets
- Buffer management for compression

---

## Part 6: Error Handling

```rust
/// Write error types
#[derive(Debug)]
pub enum WriteError {
    /// I/O error
    IoError(std::io::Error),
    /// Invalid data type
    InvalidDataType { channel: String, expected: u8, actual: u8 },
    /// Data length mismatch
    DataLengthMismatch { channel: String, expected: usize, actual: usize },
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
    InvalidState { current: WriterState, required: WriterState },
}

impl From<std::io::Error> for WriteError {
    fn from(e: std::io::Error) -> Self {
        WriteError::IoError(e)
    }
}
```

---

## Part 7: Cargo Feature Flags

```toml
[features]
default = ["read"]
read = []
write = []
compression = ["write", "flate2"]
streaming = ["write"]

[dependencies]
# Existing dependencies...
flate2 = { version = "1.0", optional = true }
```

---

## Part 8: Success Criteria

1. **Functional Requirements**
   - Can write valid MF4 files readable by existing readers
   - Round-trip: Write → Read → Write produces identical data
   - Support all basic data types (UINT, INT, FLOAT)

2. **Performance Requirements**
   - Streaming write: < 10ms overhead per record
   - Compression ratio: > 50% for typical measurement data
   - Memory usage: < 100MB for streaming writes regardless of file size

3. **Quality Requirements**
   - > 90% test coverage for write module
   - All public APIs documented with examples
   - No data corruption in edge cases (power loss, early termination)

---

## Appendix A: Block Layout Reference

```
+------------------+
| ID Block         |  Offset: 0x00
| - Magic: "MDF   "|
| - Version        |
| - ...            |
+------------------+
| HD Block         |  Offset: 0x40
| - Links: DG, ... |
| - Time stamp     |
| - ...            |
+------------------+
| DG Block 1       |  Offset: varies
| - Links: CG, ... |
+------------------+
| CG Block 1       |
| - Links: CN, ... |
| - Record size    |
+------------------+
| CN Block 1       |
| CN Block 2       |
| ...              |
+------------------+
| DT/DZ Block      |  Data area
+------------------+
| ... more DGs ... |
+------------------+
```

## Appendix B: Reference Files

- Existing read implementation: `src/lib.rs`, `src/components/`
- Block definitions: `config/*.toml`
- Test data: `test/*.mf4`
- Write module: `src/writer/`
- Write tests: `src/writer/write_test.rs`
- Examples: `examples/compression_demo.rs`, `examples/bench_stream_write.rs`, `examples/generate_test_files.rs`

---

## Appendix C: Implementation Status (as of 2026-04-10)

### Completed Features

| Feature | Status | Notes |
|---------|--------|-------|
| **MF4 File Creation** | ✅ Done | ID + HD blocks, proper header structure |
| **ID/HD Block Write** | ✅ Done | File identification and header with metadata |
| **DG Block Write** | ✅ Done | Data Group with linked CG/data blocks |
| **CG Block Write** | ✅ Done | Channel Group with record size, cycle count |
| **CN Block Write** | ✅ Done | Channel with bit offset, data type, unit |
| **DT Block Write** | ✅ Done | Uncompressed data blocks |
| **DZ Block Write** | ✅ Done | Compressed data blocks (0 links per spec, 48+data layout) |
| **DL Block Write** | ✅ Done | Data List linking multiple DT/DZ blocks with cumulative offsets |
| **HL Block Write** | ✅ Done | Header List for compressed DL chains (40 bytes) |
| **TX/MD Block Write** | ✅ Done | Text and metadata blocks |
| **CC Block Write** | ✅ Done | Conversion blocks (linear, text tables, etc.) |
| **SI Block Write** | ✅ Done | Source Information blocks |
| **One-time Write** | ✅ Done | `Mf4Builder` API |
| **Streaming Write** | ✅ Done | `Mf4StreamWriter` with compact and stream modes |
| **DL-Chained Stream Write** | ✅ Done | Record-aligned chunking with 4MB DZ limit |
| **SimpleWriter** | ✅ Done | Ergonomic high-level wrapper for common use cases |
| **Compression** | ✅ Done | Deflate compression with configurable level/threshold |
| **All numeric types** | ✅ Done | u8/u16/u32/u64, i8/i16/i32/i64, f32/f64 |
| **Strings** | ✅ Done | UTF-8 string channels |
| **Byte arrays** | ✅ Done | Raw byte array channels |

### Not Implemented

| Feature | Status | Notes |
|---------|--------|-------|
| Transpose + Deflate compression | ❌ Not started | Write support (read works) |
| CA Block Write | ❌ Not started | Channel array blocks |
| UTF-16 strings | ❌ Not started | Low priority |

### Block Hierarchy (Write Modes)

```
Compact mode (single block):
  DG.dg_data ──→ DT
  DG.dg_data ──→ DZ          (if compressed)

Stream mode (DL-chained blocks):
  DG.dg_data ──→ DL ──→ DT₁ ──→ DT₂ ──→ ... ──→ DTₙ

Stream mode + Compression:
  DG.dg_data ──→ HL ──→ DL ──→ DZ₁ ──→ DZ₂ ──→ ... ──→ DZₙ
                                 │
                           Each DZ ≤ 4MB uncompressed
                           Aligned to record boundaries
```

### API Layers

```
┌─────────────────────────────────────────────────┐
│  SimpleWriter  (3 steps: build → write → done)  │  ← Ergonomic
├─────────────────────────────────────────────────┤
│  Mf4StreamWriter  (start_record/set/flush)      │  ← Full control
│  ChannelGroupDefBuilder  (typed channel helpers) │
│  StreamingConfig  (block_size, compression)      │
├─────────────────────────────────────────────────┤
│  Mf4Builder  (one-time write)                   │  ← Batch write
├─────────────────────────────────────────────────┤
│  BlockWriter  (write_dg_block, write_cn_block)  │  ← Low-level
│  HlBlock, DlBlock, DzBlock                      │
└─────────────────────────────────────────────────┘
```

### Performance (Release Build, 4 × f64 Channels)

| Configuration | Throughput | File Size |
|---------------|-----------|-----------|
| Stream, uncompressed, 1M samples | ~634K rec/s | 30.5 MB |
| Stream, compressed, 1M samples | ~132K rec/s | 20.0 MB |
| Compact, uncompressed, 1M samples | ~549K rec/s | 30.5 MB |
| Compact, compressed, 1M samples | ~309K rec/s | 20.0 MB |
| SimpleWriter vs Full API overhead | < 3% | identical |

### Test Coverage

| Category | Test Count |
|----------|-----------|
| One-time write (Mf4Builder) | 25 |
| DL-chain stream write | 13 |
| Record alignment | 3 |
| Ergonomic API (SimpleWriter + convenience) | 9 |
| Integration (write → read roundtrip) | 34 |
| **Total** | **84** |

### Commits

1. `feat(write): implement MF4 file write functionality` — Core write with Mf4Builder + Mf4StreamWriter
2. `feat(writer): add DL-chained stream write with HL/DZ support` — DL-linked DT/DZ blocks
3. `perf(writer): optimize stream write hot path` — O(1) lookups, zero-copy, buffer reuse
4. `feat(writer): add SimpleWriter ergonomic API` — High-level wrapper + convenience methods
