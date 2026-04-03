# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

mf4_parse is a Rust library for reading MF4 (Measurement Data Format 4) files, which are used in automotive measurement and calibration systems (ASAM MDF standard). The library supports reading channel data, time stamps, and various data types including float, tex
      t, array, and composed data.

## Build and Test Commands

```bash
# Build the library
cargo build

# Build in release mode
   cargo build --release

   # Run all tests
   cargo test

# Run a specific test
cargo test test_mdf_new

# Run a specific test module
cargo test --lib parser_test

   # Run the CLI binary
cargo run --bin mf4_parse_cli
```
## Architecture

The codebase follows a hierarchical structure that mirrors the MF4 file format:

### Core Parsing Layers

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
   - `dx.rs` - Data blocks (DT/DV/DZ/DL): Raw data storage
   - `si.rs` - Source Information

3. **Parser Layer** (`lib.rs:parser` module)
   - `Mf4Wrapper`: Main user-facing API
   - `Mdf`: Internal structure holding all DataGroups
   - `MdfInfo`: Header/metadata extraction

### Configuration

Block structure definitions are stored as TOML files in `config/` (e.g., `dg.toml`, `cn.toml`). These define each block's ID, links, and data fields. The config files are embedded at compile time via `rust_embed`.

### Data Flow
```rust

MF4 File -> Mf4Wrapper::new()
-> Mdf::new() parses header and spawns threads for DataGroups
-> DataGroup::new_unchecked() parses each DG in parallel
-> ChannelGroup::new() -> Channel::new()
-> Data access via Mf4Wrapper::get_channel_data()
```

### Key Types

- `DataValue`: Enum holding typed channel data (REAL, UINT8, STRINGS, STRUCT, etc.)
- `ChannelLink`: Reference triple (Channel, ChannelGroup, DataGroup) for efficient lookups
- `VirtualBuf`: Trait for reading data blocks, handles both regular and compressed (DZ) blocks

### Memory Strategy

- Files > 2GB use memory mapping (`memmap2`)
- Smaller files are loaded into `Vec<u8>`
- Channel cache and master cache optimize repeated data access

## WASM Support

The library compiles to `wasm32` target with `wasm-bindgen` support. The `get_mf4_channels` function is exported for JavaScript use.

## Test Data

Test MF4 files are in `test/`. Tests use `rstest` for parameterized testing and `lazy_static` for shared test file loading.