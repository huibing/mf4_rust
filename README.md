# rust_mf4

A Rust library for reading and writing MF4 (Measurement Data Format 4) files, which are used in automotive measurement and calibration systems (ASAM MDF standard).

Standard reference: https://www.asam.net/standards/detail/mdf/wiki/
Demo MF4 files can be accessed from https://www.asam.net/standards/detail/mdf/

Recent changes (2026-04-08)
- feat(sort): add MF4 file sort feature — adds ability to sort MF4 file blocks for deterministic reading and merging.
- fix(cc): normalize algebraic expr math functions — improved normalization of algebraic expressions used in conversion (CC) blocks.
- fix(writer): correct multi-DG layout and data-type constants — fixes layout issues when writing multiple DataGroups and corrects data-type constant handling.

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
- Support all numeric types (u8/u16/u32/u64, i8/i16/i32/i64, f32/f64)
- Support strings and byte arrays
- Support compression (Deflate, Transpose + Deflate)
- Proper MF4 block structure (ID, HD, DG, CG, CN, TX, DT/DZ blocks)

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
use mf4_parse::writer::{Mf4StreamWriter, Mf4Metadata, StreamingDataGroup, ChannelGroupDef, ChannelDef};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create streaming writer
    let mut writer = Mf4StreamWriter::new(
        PathBuf::from("streaming.mf4"),
        Mf4Metadata::default()
    )?;

    // 2. Define channel structure
    let time_def = ChannelDef::new_master("time");
    let temp_def = ChannelDef::new("Temperature")
        .data_type(5)  // FLOAT64
        .unit("°C");

    let cg_def = ChannelGroupDef::new()
        .name("Measurement")
        .master(time_def)
        .channel(temp_def);

    // 3. Add data group and finalize structure
    let dg = StreamingDataGroup::new(cg_def);
    writer.add_data_group(dg)?;
    writer.finalize_structure()?;

    // 4. Write data incrementally
    for i in 0..100 {
        let time = i as f64 * 0.01;
        let temp = 20.0 + (i as f64 * 0.1).sin();

        writer.append_record("time", &time)?;
        writer.append_record("Temperature", &temp)?;
        writer.flush_record()?;
    }

    // 5. Finalize and close
    writer.finalize()?;

    Ok(())
}
```

More examples are available in the `src/main.rs` file.
