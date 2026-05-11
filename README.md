# rust_mf4

A Rust library for reading and writing MF4 (Measurement Data Format 4) files, which are used in automotive measurement and calibration systems (ASAM MDF standard).

Standard reference: https://www.asam.net/standards/detail/mdf/wiki/
Demo MF4 files can be accessed from https://www.asam.net/standards/detail/mdf/

Recent changes (2026-04-03 ~ 2026-04-10)

**2026-04-10**
- feat(writer): add SimpleWriter ergonomic API — high-level wrapper for common single-channel-group use case.
- feat(writer): add DL-chained stream write with HL/DZ support — streaming data stored as DL-linked DT/DZ blocks.
- perf(writer): optimize stream write hot path — O(1) channel lookup, zero-copy range computation, buffer reuse.

**2026-04-08**
- feat(sort): add MF4 file sort feature — converts unsorted MF4 files (multiple ChannelGroups per DataGroup) into sorted format (one ChannelGroup per DataGroup).
- fix(cc): normalize algebraic expr math functions — improved normalization of algebraic expressions used in CC blocks.

**2026-04-07**
- feat(write): add SI block support for ChannelGroupBuilder — enables source information attachment to channels.

**2026-04-06**
- fix: correct CC block data layout for Value2Text conversion — enables text-based channel value lookups.

**2026-04-04 ~ 2026-04-05**
- feat(write): implement MF4 file write functionality — complete write support with Mf4Builder (one-time) and Mf4StreamWriter (streaming).
- perf: improve write efficiency and reduce memory allocations.
- fix: correct data writing for multiple channel groups.

## Features

### Reading
- Read header info
- Read channel info
- Read float data from mf4 file
- Read text data from mf4 file
- Read array data from mf4 file
- Read composed data from mf4 file
- Read mf4 file with compressed data blocks (DZ blocks)
- Support transpose + deflate compression method

### Writing
- **One-time Write Mode**: Create complete MF4 files in a single operation using `Mf4Builder`
- **Streaming Write Mode**: Incrementally append data using `Mf4StreamWriter`
  - **Compact mode**: Single DT block (uncompressed), or DZ chain ≤4MB per block (compressed)
  - **Stream mode**: Data split into DL-chained DT blocks for efficient buffering
  - **Compressed stream mode**: DG → HL → DL → [DZ₁, DZ₂, ...], each DZ ≤ 4MB uncompressed
- **SimpleWriter**: High-level ergonomic wrapper for common single-channel-group use cases
- Support all numeric types (u8/u16/u32/u64, i8/i16/i32/i64, f32/f64)
- Support strings and byte arrays
- Support compression (Deflate, Transpose + Deflate)
- Proper MF4 block structure (ID, HD, DG, CG, CN, TX, DT/DZ/DL/HL blocks)

### Sorting
- Convert unsorted MF4 files to sorted format
- Unsorted: one DataGroup contains multiple ChannelGroups with interleaved records
- Sorted: each DataGroup contains exactly one ChannelGroup with contiguous data
- Useful for deterministic reading and merging MF4 files

## Feature Flags

The library uses feature flags to control which functionality is compiled:

| Flag | Description |
|------|-------------|
| `read` | Enable reading functionality (enabled by default) |
| `write` | Enable one-time write functionality via `Mf4Builder` |
| `streaming` | Enable streaming write functionality via `Mf4StreamWriter` (implies `write`) |
| `compression` | Enable compression support (implies `write`) |

Default features: `["read"]`

## Un-supported Features

- Invalid bit flag processing
- Bitfield text table conversion
- Inverse conversion
- CG and DG-template CA block
- Sample reduction block
- LD/FH/CH/AT blocks

Most of the above features are not supported because it is difficult to obtain MF4 files with these features for development and testing. In practice, these features are rarely used by tools that generate MF4 files.

## Install

Currently, this lib is not registered to crates.io.
You can clone this repo and use it locally.

```toml
## Cargo.toml
[dependencies]
mf4_parse = { path = "/local/path/to/rust_mf4_repo" }

# To enable write functionality:
# mf4_parse = { path = "/local/path/to/rust_mf4_repo", features = ["write"] }

# To enable streaming write:
# mf4_parse = { path = "/local/path/to/rust_mf4_repo", features = ["streaming"] }

# To enable all features:
# mf4_parse = { path = "/local/path/to/rust_mf4_repo", features = ["write", "streaming", "compression"] }
```

Alternatively, you can specify this repo as a git dependency:

```toml
[dependencies]
mf4_parse = { git = "https://github.com/huibing/rust_mf4.git" }
```

## Examples

### Reading MF4 Files

Here is a simple example without proper error handling:

```rust
use mf4_parse::Mf4Wrapper;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mf4 = Mf4Wrapper::new(PathBuf::from("./test_data/test.mf4"))?;
    println!("Header time stamp: {:?}", mf4.get_time_stamp());
    for (index, ch_name) in mf4.get_channel_names().iter().enumerate() {
        println!("{}th channel name: {:?}", index, ch_name);
    }
    println!("Channel1 data: {:?}", mf4.get_channel_data("Channel1").unwrap());
    println!("channel1's time stamp data: {:?}", mf4.get_channel_master_data("Channel1").unwrap());
    Ok(())
}
```

### Reading Textual Channel Data

For channels whose values are represented as text — including string channels
(`cn_data_type` 6–9), VTAB channels (CC type 7), and VRange-to-text channels
(CC type 8) — use `get_channel_text_data`, which returns `Vec<String>` directly:

```rust
use mf4_parse::Mf4Wrapper;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mf4 = Mf4Wrapper::new::<fn(f64)>(PathBuf::from("gear_log.mf4"), None)?;

    // Works for VTAB / VRange-to-text channels:
    if let Some(labels) = mf4.get_channel_text_data("gear") {
        for (i, label) in labels.iter().enumerate() {
            println!("sample {i}: {label}");   // e.g. "First", "Second", "N/A"
        }
    }

    // Works for string channels too:
    if let Some(msgs) = mf4.get_channel_text_data("error_msg") {
        println!("{:?}", msgs);
    }

    // Numeric channels return None:
    assert!(mf4.get_channel_text_data("rpm").is_none());

    Ok(())
}
```

You can also test a `DataValue` directly:

```rust
use mf4_parse::DataValue;

let data = mf4.get_channel_data("gear").unwrap();
if data.is_text() {
    let texts: Vec<String> = data.into_text().unwrap();
    println!("{:?}", texts);
}
```

### Writing MF4 Files (One-time Write)

Use `Mf4Builder` to create complete MF4 files in a single operation:

```rust
use mf4_parse::writer::{Mf4Builder, Mf4Metadata, DataGroupBuilder, ChannelGroupBuilder, ChannelBuilder};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create metadata
    let metadata = Mf4Metadata::new()
        .with_author("Test User")
        .with_organization("Test Org");

    // 2. Create builder
    let mut builder = Mf4Builder::new(metadata);

    // 3. Define channels
    let time_channel = ChannelBuilder::new_master("time");
    let temp_channel = ChannelBuilder::new("Temperature")
        .data_type(5)      // FLOAT64
        .unit("°C")
        .comment("Engine temperature");

    // 4. Create channel group
    let cg = ChannelGroupBuilder::new()
        .name("EngineData")
        .master(time_channel)
        .channel(temp_channel);

    // 5. Create data group
    let dg = DataGroupBuilder::new().channel_group(cg);
    builder.add_data_group(dg);

    // 6. Add data
    let time_data: Vec<f64> = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let temp_data: Vec<f64> = vec![20.0, 21.5, 23.0, 24.5, 26.0];

    builder.set_channel_data("time", &time_data)?;
    builder.set_channel_data("Temperature", &temp_data)?;

    // 7. Write to file
    builder.write(PathBuf::from("output.mf4"))?;

    Ok(())
}
```

### Writing with Compression

```rust
use mf4_parse::writer::{Mf4Builder, Mf4Metadata, CompressionConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

### Streaming Write

Use `Mf4StreamWriter` for incremental data append (useful for real-time data acquisition):

```rust
use mf4_parse::writer::stream_writer::{
    ChannelGroupDefBuilder, Mf4StreamWriter, StreamingConfig, StreamingDataGroup,
};
use mf4_parse::writer::Mf4Metadata;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create streaming writer with configuration
    let metadata = Mf4Metadata::new().with_author("My App");
    let config = StreamingConfig::new()
        .with_compression_level(6);  // Enable zlib compression
    let mut writer = Mf4StreamWriter::with_config(
        PathBuf::from("streaming.mf4"), metadata, config
    )?;

    // 2. Define channel structure using convenience methods
    let cg = ChannelGroupDefBuilder::new()
        .name("Measurement")
        .with_time_channel("time")       // Master channel (f64, unit "s")
        .add_f64_channel("voltage", "V") // Data channel
        .add_f64_channel("current", "A") // Data channel
        .build()?;

    // 3. Add data group and finalize structure
    writer.add_data_group(StreamingDataGroup::new(cg)?)?;
    writer.finalize_structure()?;

    // 4. Write data incrementally
    for i in 0..1000 {
        let time = i as f64 * 0.001;
        writer.start_record(0, 0)?;
        writer.set_channel_value("time", time)?;
        writer.set_channel_value("voltage", 3.3 + (time * 10.0).sin())?;
        writer.set_channel_value("current", 0.5 + (time * 5.0).cos() * 0.1)?;
        writer.flush_record()?;
    }

    // 5. Finalize — false = stream mode (DL-chained blocks)
    //              true  = compact mode (single DT for uncompressed;
    //                      DZ chain ≤4MB/block for compressed data > 4MB)
    writer.finalize_with_compact(false)?;

    Ok(())
}
```

#### Shorthand with `write_record`

For single-DG/single-CG files with all f64 channels, use the `write_record` shorthand:

```rust
// After setup (steps 1-3 above)...
// Values in channel definition order: [time, voltage, current]
writer.write_record(&[0.0, 3.3, 0.5])?;
writer.write_record(&[0.001, 3.4, 0.6])?;
writer.finalize_with_compact(false)?;
```

### SimpleWriter (Ergonomic API)

For the common case of a single channel group with f64 channels, `SimpleWriter` reduces
the entire setup to a fluent builder:

```rust
use mf4_parse::writer::SimpleWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = SimpleWriter::new("output.mf4")
        .author("My App")
        .time_channel("time", "s")
        .f64_channel("voltage", "V")
        .f64_channel("current", "A")
        .compression(6)      // zlib level 6
        .stream_mode()        // DL-chained blocks (default)
        .build()?;

    for i in 0..10000 {
        let t = i as f64 * 0.001;
        writer.write_record(&[t, 3.3 + t.sin(), 0.5 + t.cos() * 0.1])?;
    }

    writer.finalize()?;
    Ok(())
}
```

Available channel types: `time_channel`, `f64_channel`, `f32_channel`,
`u8_channel`, `u16_channel`, `u32_channel`, `u64_channel`,
`i16_channel`, `i32_channel`.

Use `.compact_mode()` instead of `.stream_mode()` for single-DT output (uncompressed).
For compressed data, both modes produce a DZ chain (≤4MB per DZ block, per MDF4 protocol).

### Writing Value-to-Text (VTAB) Channels

VTAB channels store raw numeric values in the data section and use a CC block
(conversion type 7 or 8) to map raw values to display strings.

#### Streaming write — CC type 7 (exact key match)

```rust
use mf4_parse::writer::stream_writer::{
    ChannelDef, ChannelGroupDefBuilder, Mf4StreamWriter, StreamingConfig, StreamingDataGroup,
};
use mf4_parse::writer::Mf4Metadata;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cg = ChannelGroupDefBuilder::new()
        .name("Gear")
        .with_time_channel("time")
        // u8 channel; keys 1/2/3 display "1st"/"2nd"/"3rd"; unmatched → "N/A"
        .add_vtab_channel(
            "gear", 0, 8,
            vec![1.0, 2.0, 3.0],
            vec!["1st".into(), "2nd".into(), "3rd".into()],
            "N/A",
        )
        .build()?;

    let mut writer = Mf4StreamWriter::with_config(
        PathBuf::from("gear.mf4"), Mf4Metadata::new(), StreamingConfig::new(),
    )?;
    writer.add_data_group(StreamingDataGroup::new(cg)?)?;
    writer.finalize_structure()?;

    for (t, gear) in [(0.0, 1u8), (0.1, 2), (0.2, 3), (0.3, 1)] {
        writer.start_record(0, 0)?;
        writer.set_channel_value("time", t)?;
        writer.set_channel_value("gear", gear as f64)?;
        writer.flush_record()?;
    }
    writer.finalize()?;
    Ok(())
}
```

#### Streaming write — CC type 8 (range match)

```rust
let cg = ChannelGroupDefBuilder::new()
    .name("TempBand")
    .with_time_channel("time")
    // f64 channel; ranges [0,30) → "Cold", [30,70) → "Warm", [70,100] → "Hot"
    .add_vrange_channel(
        "temp_band", 4, 64,
        vec![(0.0, 30.0), (30.0, 70.0), (70.0, 100.0)],
        vec!["Cold".into(), "Warm".into(), "Hot".into()],
        "OOB",
    )
    .build()?;
```

#### SimpleWriter — vtab / vrange shorthand

```rust
use mf4_parse::writer::SimpleWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = SimpleWriter::new("status.mf4")
        .time_channel("time", "s")
        // CC type 7 — exact key match (u8 raw storage)
        .vtab_u8_channel(
            "status",
            vec![0.0, 1.0, 2.0],
            vec!["Off".into(), "On".into(), "Fault".into()],
            "N/A",
        )
        // CC type 8 — range match (u8 raw storage)
        .vrange_u8_channel(
            "level",
            vec![(0.0, 49.0), (50.0, 100.0)],
            vec!["Low".into(), "High".into()],
            "N/A",
        )
        .build()?;

    writer.write_record(&[0.0, 0.0, 25.0])?;   // status=Off, level=Low
    writer.write_record(&[0.1, 1.0, 75.0])?;   // status=On,  level=High
    writer.finalize()?;
    Ok(())
}
```

#### One-time write (`Mf4Builder`) — CC type 7

```rust
use mf4_parse::writer::{
    Mf4Builder, Mf4Metadata, DataGroupBuilder, ChannelGroupBuilder, ChannelBuilder,
    ConversionBuilder,
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gear_channel = ChannelBuilder::new("gear")
        .data_type(0)    // UINT LE
        .bit_count(8)
        .conversion(ConversionBuilder::value2text(
            vec![1.0, 2.0, 3.0],
            vec!["1st".into(), "2nd".into(), "3rd".into()],
            "N/A".into(),
        ))
        .build()?;

    let cg = ChannelGroupBuilder::new()
        .name("Gear")
        .master(ChannelBuilder::new_master_time("time"))
        .channel(gear_channel)
        .build()?;

    let mut builder = Mf4Builder::new(Mf4Metadata::new());
    builder.add_data_group(DataGroupBuilder::new().channel_group(cg));
    builder.set_channel_data("time", &[0.0_f64, 0.1, 0.2])?;
    builder.set_channel_data("gear", &[1.0_f64, 2.0, 3.0])?;
    builder.write(PathBuf::from("gear_block.mf4"))?;
    Ok(())
}
```

### Sorting MF4 Files

Convert unsorted MF4 files (multiple ChannelGroups per DataGroup) to sorted format:

```rust
use mf4_parse::sort::sort_mf4;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sort an unsorted MF4 file
    sort_mf4(
        PathBuf::from("unsorted.mf4"),
        PathBuf::from("sorted.mf4")
    )?;

    println!("MF4 file sorted successfully!");
    Ok(())
}
```

More examples are available in the `src/main.rs` file.
