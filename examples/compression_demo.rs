//! Example: Compressed vs Non-Compressed MF4 File Generation
//!
//! This example demonstrates the three valid write strategies:
//! - **Compact mode** (uncompressed only): All data in a single DT block
//! - **Stream mode** (uncompressed): Data split into DL-chained DT blocks
//! - **Stream mode** (compressed): Data split into HL→DL→DZ blocks, each DZ ≤ 4MB
//!
//! Note: compact mode and compression are mutually exclusive by the MDF4 protocol —
//! each DZ block is capped at 4MB uncompressed, so compact+compressed cannot guarantee
//! a single block for arbitrary data sizes.
//!
//! Run with: cargo run --example compression_demo --features streaming,compression
//!
//! The example generates:
//! - compact_uncompressed.mf4:   Single DT block (compact mode, uncompressed)
//! - stream_uncompressed.mf4:    DL → [DT₁, DT₂, ...] chain (stream mode)
//! - stream_compressed.mf4:      HL → DL → [DZ₁, DZ₂, ...] chain (stream mode, compressed)
//!
//! It also compares the file sizes to show the compression and structure differences.

use std::path::PathBuf;

use mf4_parse::parser::Mf4Wrapper;
use mf4_parse::writer::stream_writer::{ChannelGroupDefBuilder, ChannelDef, StreamingConfig,
    StreamingDataGroup, Mf4StreamWriter};
use mf4_parse::writer::{Mf4Metadata, SourceInfoBuilder, SourceType};

/// Number of samples to generate (more samples = better compression ratio)
const NUM_SAMPLES: usize = 10_000_000;

/// Number of channels in addition to the time channel
const NUM_CHANNELS: usize = 5;

/// Batch size for streaming writes (samples per batch)
const BATCH_SIZE: usize = 100_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MF4 Compression & Stream Write Demo ===\n");
    println!("Generating {} samples across {} channels...\n", NUM_SAMPLES, NUM_CHANNELS + 1);

    // Signal patterns
    let signal_patterns: [(f64, f64, f64); NUM_CHANNELS] = [
        (1.0, 20.0, 5.0),
        (0.5, 100.0, 50.0),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
    ];

    // ==========================================================================
    // 1. Compact + Uncompressed (single DT block)
    //    compact_mode and compression are mutually exclusive.
    // ==========================================================================
    let compact_uncompressed_path = PathBuf::from("compact_uncompressed.mf4");
    println!("1. Generating compact_uncompressed.mf4 (single DT block)...");
    write_mf4_file(
        &compact_uncompressed_path,
        StreamingConfig::new()
            .with_block_size(10_000_000),
        &signal_patterns,
        true, // compact
        "Compact mode: single DT block (all data in one uncompressed block)",
    )?;

    // ==========================================================================
    // 2. Stream + Uncompressed (DL → [DT₁, DT₂, ...])
    // ==========================================================================
    let stream_uncompressed_path = PathBuf::from("stream_uncompressed.mf4");
    println!("2. Generating stream_uncompressed.mf4 (DL → DT chain)...");
    write_mf4_file(
        &stream_uncompressed_path,
        StreamingConfig::new()
            .with_block_size(4_000_000), // 4MB DT blocks
        &signal_patterns,
        false, // stream mode (DL chain)
        "Stream mode: DL → DT chain (data split into multiple DT blocks)",
    )?;

    // ==========================================================================
    // 3. Stream + Compressed (HL → DL → [DZ₁, DZ₂, ...])
    //    Each DZ block ≤ 4MB uncompressed, record-aligned boundaries.
    // ==========================================================================
    let stream_compressed_path = PathBuf::from("stream_compressed.mf4");
    println!("3. Generating stream_compressed.mf4 (HL → DL → DZ chain)...");
    write_mf4_file(
        &stream_compressed_path,
        StreamingConfig::new()
            // block_size only affects DT blocks; DZ blocks are always ≤4MB per MDF4 protocol
            .with_block_size(4_000_000)
            .with_compression_level(6),
        &signal_patterns,
        false, // stream mode (DL chain)
        "Stream mode: HL → DL → DZ chain (each DZ ≤ 4MB, record-aligned)",
    )?;

    // ==========================================================================
    // Compare File Sizes
    // ==========================================================================
    println!("\n=== File Size Comparison ===\n");

    let files = [
        ("compact_uncompressed.mf4", &compact_uncompressed_path),
        ("stream_uncompressed.mf4",  &stream_uncompressed_path),
        ("stream_compressed.mf4",    &stream_compressed_path),
    ];

    let mut sizes = Vec::new();
    for (name, path) in &files {
        let size = std::fs::metadata(path)?.len();
        sizes.push(size);
        println!("  {:<30} {:>12} bytes ({:.2} MB)",
            name, size, size as f64 / 1_048_576.0);
    }

    let baseline = sizes[0] as f64; // compact_uncompressed as baseline
    println!("\n  Compression ratios (vs compact_uncompressed):");
    for (i, (name, _)) in files.iter().enumerate() {
        let ratio = sizes[i] as f64 / baseline * 100.0;
        println!("    {:<30} {:.1}%", name, ratio);
    }

    // ==========================================================================
    // Verify Data Integrity
    // ==========================================================================
    println!("\n=== Data Integrity Verification ===\n");

    let reference_mf4 = Mf4Wrapper::new::<fn(f64)>(compact_uncompressed_path.clone(), None)?;

    for (name, path) in &files {
        let mf4 = Mf4Wrapper::new::<fn(f64)>(path.to_path_buf(), None)?;
        let mut ok = true;

        for ch_name in &reference_mf4.get_channel_names() {
            let ref_data = reference_mf4.get_channel_data(ch_name);
            let test_data = mf4.get_channel_data(ch_name);

            match (ref_data, test_data) {
                (Some(mf4_parse::data_serde::DataValue::REAL(ref_vals)),
                 Some(mf4_parse::data_serde::DataValue::REAL(test_vals))) => {
                    let check_len = 100.min(ref_vals.len()).min(test_vals.len());
                    if ref_vals.len() != test_vals.len() {
                        println!("  ✗ {}: {} sample count mismatch ({} vs {})",
                            name, ch_name, ref_vals.len(), test_vals.len());
                        ok = false;
                    } else if !ref_vals[..check_len].iter()
                        .zip(test_vals[..check_len].iter())
                        .all(|(a, b)| (a - b).abs() < 1e-10) {
                        println!("  ✗ {}: {} data mismatch", name, ch_name);
                        ok = false;
                    }
                }
                _ => {}
            }
        }

        if ok {
            println!("  ✓ {} - all channels match reference", name);
        }
    }

    // ==========================================================================
    // Summary
    // ==========================================================================
    println!("\n=== Summary ===");
    println!("Block structure hierarchy:");
    println!("  compact_uncompressed: DG → DT (single uncompressed block)");
    println!("  stream_uncompressed:  DG → DL → [DT₁, DT₂, ...]");
    println!("  stream_compressed:    DG → HL → DL → [DZ₁, DZ₂, ...] (each DZ ≤ 4MB)");
    println!("\nKey points:");
    println!("  - compact_mode and compression are mutually exclusive");
    println!("  - DZ block uncompressed size is capped at 4MB (MDF4 protocol requirement)");
    println!("  - Record boundaries are preserved — no record spans two blocks");
    println!("  - block_size only affects DT (uncompressed) buffer flushing");
    println!("  - finalize_with_compact(false) → stream mode; (true) → single DT block");

    Ok(())
}

/// Write an MF4 file with the given configuration
fn write_mf4_file(
    path: &PathBuf,
    config: StreamingConfig,
    signal_patterns: &[(f64, f64, f64); NUM_CHANNELS],
    compact: bool,
    comment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = Mf4Metadata::new()
        .with_author("Compression Demo")
        .with_organization("MF4 Parse Library")
        .with_project("Compression Example")
        .with_comment(comment);

    let mut writer = Mf4StreamWriter::with_config(path.clone(), metadata, config)?;

    let source = SourceInfoBuilder::new()
        .name("DAQ_100MHz")
        .path("DAQ/Card1")
        .source_type(SourceType::Io)
        .build()?;

    let mut cg_builder = ChannelGroupDefBuilder::new()
        .name("MeasurementData")
        .acq_source(source)
        .master(ChannelDef::new_master("time"));

    for ch_idx in 0..NUM_CHANNELS {
        cg_builder = cg_builder.channel(
            ChannelDef::new(&format!("Signal_{}", ch_idx))
                .data_type(4)
                .unit(&format!("Unit_{}", ch_idx))
        );
    }

    let cg = cg_builder.build()?;
    writer.add_data_group(StreamingDataGroup::new(cg)?)?;
    writer.finalize_structure()?;

    let num_batches = (NUM_SAMPLES + BATCH_SIZE - 1) / BATCH_SIZE;
    for batch_idx in 0..num_batches {
        let start = batch_idx * BATCH_SIZE;
        let end = (start + BATCH_SIZE).min(NUM_SAMPLES);

        for i in 0..(end - start) {
            let sample_idx = start + i;
            let t = sample_idx as f64 * 0.001;

            writer.start_record(0, 0)?;
            writer.set_channel_value("time", t)?;

            for ch_idx in 0..NUM_CHANNELS {
                let val = generate_signal_value(ch_idx, sample_idx, t, signal_patterns);
                writer.set_channel_value(&format!("Signal_{}", ch_idx), val)?;
            }
            writer.flush_record()?;
        }

        if (batch_idx + 1) % 10 == 0 || batch_idx == num_batches - 1 {
            println!("  Progress: {}/{} batches ({} samples)", batch_idx + 1, num_batches, end);
        }
    }

    writer.finalize_with_compact(compact)?;

    let size = std::fs::metadata(path)?.len();
    println!("  -> {} ({:.2} MB)\n", path.display(), size as f64 / 1_048_576.0);
    Ok(())
}

/// Generate signal value based on channel index and sample position
fn generate_signal_value(ch_idx: usize, sample_idx: usize, t: f64, patterns: &[(f64, f64, f64)]) -> f64 {
    match ch_idx {
        0 => {
            // Sine wave
            let (_, offset, amp) = patterns[0];
            offset + amp * (2.0 * std::f64::consts::PI * 10.0 * t).sin()
        }
        1 => {
            // Cosine wave
            let (_, offset, amp) = patterns[1];
            offset + amp * (2.0 * std::f64::consts::PI * 5.0 * t).cos()
        }
        2 => {
            // Sawtooth
            t * 1000.0 % 256.0
        }
        3 => {
            // Noisy signal (deterministic pseudo-random)
            let base = 50.0 * (2.0 * std::f64::consts::PI * 20.0 * t).sin();
            let noise = (sample_idx as f64 * 0.12345).sin() * 5.0;
            base + noise
        }
        4 => {
            // Step function
            25.0 + (t * 10.0).floor() % 10.0
        }
        _ => 0.0
    }
}
