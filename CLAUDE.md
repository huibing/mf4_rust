# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

mf4_parse is a Rust library for reading and writing MF4 (Measurement Data Format 4) files, which are used in automotive measurement and calibration systems (ASAM MDF standard). The library supports reading channel data, time stamps, and various data types including float, text, array, and composed data. It also provides streaming and one-time write modes with optional compression.

## Build and Test Commands

```bash
# Build the library
cargo build

# Build with write features
cargo build --features "streaming,compression"

# Build in release mode
cargo build --release

# Run all tests
cargo test

# Run all tests including write/streaming/compression
cargo test --features "streaming,compression"

# Run a specific test
cargo test test_mdf_new

# Run a specific test module
cargo test --lib parser_test

# Run the CLI binary
cargo run --bin mf4_parse_cli

# Run benchmark example
cargo run --release --example bench_stream_write --features "streaming,compression"
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `read` | Reading functionality (enabled by default) |
| `write` | One-time write via `Mf4Builder` |
| `streaming` | Streaming write via `Mf4StreamWriter` (implies `write`) |
| `compression` | Compression support for DZ blocks (implies `write`) |

## Architecture

The codebase follows a hierarchical structure that mirrors the MF4 file format:

### Core Parsing Layers (Reading)

1. **Block Layer** (`lib.rs:block` module)
   - Low-level parsing primitives for MDF blocks
   - `BlockDesc`: TOML-defined block structure descriptors
   - `BlockInfo`: Runtime parsed block data with links and data fields

2. **Component Layer** (`src/components/`)
   - `dg.rs` - DataGroup: Top-level container owning ChannelGroups
   - `cg.rs` - ChannelGroup: Contains multiple Channels
   - `cn.rs` - Channel: Individual signal/data channel
   - `cc.rs` - Conversion: Unit conversion rules (linear, text tables, etc.)
   - `ca.rs` - ChannelArray: Array channel support
   - `dx.rs` - Data blocks (DT/DV/DZ/DL/HL): Raw data storage
   - `si.rs` - Source Information

3. **Parser Layer** (`lib.rs:parser` module)
   - `Mf4Wrapper`: Main user-facing API
   - `Mdf`: Internal structure holding all DataGroups
   - `MdfInfo`: Header/metadata extraction

### Writer Layer (`src/writer/`)

4. **Block Writer** (`block_writer.rs`)
   - Low-level block writing (ID, HD, DG, CG, CN, TX, DT, DZ, DL, HL)
   - `HlBlock`: Header List block for compressed DL chains
   - `write_dz_block()`: DZ block with 0 links per MDF4 spec
   - `write_dl_block()`: DL block with cumulative data offset array

5. **One-time Writer** (`builder.rs`)
   - `Mf4Builder`: Create complete MF4 files in a single operation
   - `DataGroupBuilder`, `ChannelGroupBuilder`, `ChannelBuilder`

6. **Streaming Writer** (`stream_writer.rs`)
   - `Mf4StreamWriter`: Incremental record-by-record writing
   - `StreamingDataGroup`: Manages data buffering and block chains
   - `DataBlockChain`: Handles DL-chained DT/DZ blocks with record-aligned chunking
   - `ChannelGroupDefBuilder`: Fluent API with typed channel helpers
   - `StreamingConfig`: Block size, compression level, threshold settings

7. **SimpleWriter** (`simple_writer.rs`)
   - `SimpleWriter` / `SimpleWriterBuilder`: Ergonomic wrapper for single-DG/single-CG files
   - Fluent builder: `.time_channel()`, `.f64_channel()`, `.compression()`, `.build()`
   - `write_record(&[f64])`: Single-call record writing

### Configuration

Block structure definitions are stored as TOML files in `config/` (e.g., `dg.toml`, `cn.toml`). These define each block's ID, links, and data fields. The config files are embedded at compile time via `rust_embed`.

### Data Flow (Reading)
```
MF4 File -> Mf4Wrapper::new()
  -> Mdf::new() parses header and spawns threads for DataGroups
  -> DataGroup::new_unchecked() parses each DG in parallel
  -> ChannelGroup::new() -> Channel::new()
  -> Data access via Mf4Wrapper::get_channel_data()
```

### Data Flow (Writing)
```
# Compact mode:  DG.dg_data -> DT (or DZ if compressed)
# Stream mode:   DG.dg_data -> DL -> [DT₁, DT₂, ..., DTₙ]
# Stream+compr:  DG.dg_data -> HL -> DL -> [DZ₁, DZ₂, ..., DZₙ]

SimpleWriter::new(path).time_channel(..).f64_channel(..).build()
  -> Mf4StreamWriter::with_config()
  -> writer.write_record(&[...])
     -> start_record() + set_channel_value() + flush_record()
     -> DataBlockChain buffers data, chunks at block_size boundary
  -> writer.finalize()
     -> DataBlockChain writes DL/HL/DT/DZ blocks
     -> Updates DG.dg_data pointer, CG.cg_cycle_count
```

### Key Types

- `DataValue`: Enum holding typed channel data (REAL, UINT8, STRINGS, STRUCT, etc.)
- `ChannelLink`: Reference triple (Channel, ChannelGroup, DataGroup) for efficient lookups
- `VirtualBuf`: Trait for reading data blocks, handles both regular and compressed (DZ) blocks
- `RecordValue`: Trait for type-safe record field writing (f64, f32, u8..u64, i8..i64)
- `RecordData`: Fixed-size byte buffer representing one record

### Memory Strategy

- Files > 2GB use memory mapping (`memmap2`)
- Smaller files are loaded into `Vec<u8>`
- Channel cache and master cache optimize repeated data access
- Stream writer uses reusable record buffer and pre-allocated shared buffer to minimize allocations

### Performance Notes

- Stream writer uses HashMap for O(1) channel lookup by name
- `active_dg_index` avoids linear scan when writing to a single data group
- `compute_record_aligned_ranges()` returns (start, end) pairs without copying data
- DZ blocks are capped at 4MB uncompressed, aligned to record boundaries
- Benchmark: ~950K rec/s uncompressed, ~475K rec/s compressed (4×f64 channels, release mode)

## WASM Support

The library compiles to `wasm32` target with `wasm-bindgen` support. The `get_mf4_channels` function is exported for JavaScript use.

## Test Data

Test MF4 files are in `test/`. Tests use `rstest` for parameterized testing and `lazy_static` for shared test file loading. Write tests are in `src/writer/write_test.rs` (84 tests covering one-time write, streaming, DL-chain, compression, alignment, and ergonomic API).