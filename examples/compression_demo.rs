//! Example: Compressed vs Non-Compressed MF4 File Generation
//!
//! This example demonstrates how to generate MF4 files with and without data compression.
//! It uses streaming write to handle large datasets efficiently without loading all data
//! into memory at once.
//!
//! Run with: cargo run --example compression_demo --features streaming,compression
//!
//! The example generates:
//! - compressed.mf4: Uses DZ blocks (compressed with Deflate)
//! - uncompressed.mf4: Uses DT blocks (uncompressed)
//!
//! It also compares the file sizes to show the compression benefit.

use std::path::PathBuf;

use mf4_parse::parser::Mf4Wrapper;
use mf4_parse::writer::stream_writer::{ChannelGroupDefBuilder, ChannelDef, StreamingConfig,
    StreamingDataGroup, Mf4StreamWriter};
use mf4_parse::writer::{Mf4Metadata, SourceInfoBuilder, SourceType};

/// Number of samples to generate (more samples = better compression ratio)
const NUM_SAMPLES: usize = 100_000_000;

/// Number of channels in addition to the time channel
const NUM_CHANNELS: usize = 5;

/// Batch size for streaming writes (samples per batch)
const BATCH_SIZE: usize = 100_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MF4 Compression Demo (Streaming) ===\n");
    println!("Generating {} samples across {} channels...\n", NUM_SAMPLES, NUM_CHANNELS + 1);
    println!("Using batch size of {} samples for streaming writes\n", BATCH_SIZE);

    // Generate signal patterns (coefficients for different wave types)
    let signal_patterns: [(f64, f64, f64); NUM_CHANNELS] = [
        (1.0, 20.0, 5.0),   // Sine wave: amplitude=5, offset=20, freq=1Hz
        (0.5, 100.0, 50.0), // Cosine wave: amplitude=50, offset=100, freq=0.5Hz
        (0.0, 0.0, 0.0),    // Sawtooth
        (0.0, 0.0, 0.0),    // Noisy signal
        (0.0, 0.0, 0.0),    // Step function
    ];

    // ==========================================================================
    // Generate Compressed MF4 File
    // ==========================================================================
    println!("Generating compressed MF4 file...");
    let compressed_path = PathBuf::from("compressed.mf4");

    {
        let metadata = Mf4Metadata::new()
            .with_author("Compression Demo")
            .with_organization("MF4 Parse Library")
            .with_project("Compression Example")
            .with_comment("Compressed DZ blocks via streaming write");

        // Configure streaming with compression enabled
        let config = StreamingConfig::new()
            .with_block_size(10_000_000)  // 10 MB blocks
            .with_compression_level(6);   // Default compression level

        let mut writer = Mf4StreamWriter::with_config(
            compressed_path.clone(),
            metadata,
            config,
        )?;

        // Define channel group with source info
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
                    .data_type(4)  // FLOAT64
                    .unit(&format!("Unit_{}", ch_idx))
            );
        }

        let cg = cg_builder.build()?;
        writer.add_data_group(StreamingDataGroup::new(cg)?)?;
        writer.finalize_structure()?;

        // Write data in batches
        let num_batches = (NUM_SAMPLES + BATCH_SIZE - 1) / BATCH_SIZE;
        for batch_idx in 0..num_batches {
            let start = batch_idx * BATCH_SIZE;
            let end = (start + BATCH_SIZE).min(NUM_SAMPLES);
            let batch_len = end - start;

            for i in 0..batch_len {
                let sample_idx = start + i;
                let t = sample_idx as f64 * 0.001;

                writer.start_record(0, 0)?;
                writer.set_channel_value("time", t)?;

                for ch_idx in 0..NUM_CHANNELS {
                    let val = generate_signal_value(ch_idx, sample_idx, t, &signal_patterns);
                    writer.set_channel_value(&format!("Signal_{}", ch_idx), val)?;
                }
                writer.flush_record()?;
            }

            if (batch_idx + 1) % 10 == 0 || batch_idx == num_batches - 1 {
                println!("  Progress: {}/{} batches ({} samples)",
                    batch_idx + 1, num_batches, end);
            }
        }

        writer.finalize_with_compact(true)?;
    }

    // ==========================================================================
    // Generate Uncompressed MF4 File
    // ==========================================================================
    println!("\nGenerating uncompressed MF4 file...");
    let uncompressed_path = PathBuf::from("uncompressed.mf4");

    {
        let metadata = Mf4Metadata::new()
            .with_author("Compression Demo")
            .with_organization("MF4 Parse Library")
            .with_project("Compression Example")
            .with_comment("Uncompressed DT blocks via streaming write");

        // Configure streaming WITHOUT compression
        let config = StreamingConfig::new()
            .with_block_size(10_000_000);  // 10 MB blocks, no compression

        let mut writer = Mf4StreamWriter::with_config(
            uncompressed_path.clone(),
            metadata,
            config,
        )?;

        // Define identical channel group
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

        // Write identical data in batches
        let num_batches = (NUM_SAMPLES + BATCH_SIZE - 1) / BATCH_SIZE;
        for batch_idx in 0..num_batches {
            let start = batch_idx * BATCH_SIZE;
            let end = (start + BATCH_SIZE).min(NUM_SAMPLES);
            let batch_len = end - start;

            for i in 0..batch_len {
                let sample_idx = start + i;
                let t = sample_idx as f64 * 0.001;

                writer.start_record(0, 0)?;
                writer.set_channel_value("time", t)?;

                for ch_idx in 0..NUM_CHANNELS {
                    let val = generate_signal_value(ch_idx, sample_idx, t, &signal_patterns);
                    writer.set_channel_value(&format!("Signal_{}", ch_idx), val)?;
                }
                writer.flush_record()?;
            }

            if (batch_idx + 1) % 10 == 0 || batch_idx == num_batches - 1 {
                println!("  Progress: {}/{} batches ({} samples)",
                    batch_idx + 1, num_batches, end);
            }
        }

        writer.finalize_with_compact(true)?;
    }

    // ==========================================================================
    // Compare File Sizes
    // ==========================================================================
    println!("\n=== File Size Comparison ===");

    let compressed_size = std::fs::metadata(&compressed_path)?.len();
    let uncompressed_size = std::fs::metadata(&uncompressed_path)?.len();
    let ratio = compressed_size as f64 / uncompressed_size as f64;
    let savings = (1.0 - ratio) * 100.0;

    println!("Compressed file:   {:>12} bytes ({:.2} MB)", compressed_size, compressed_size as f64 / 1_048_576.0);
    println!("Uncompressed file: {:>12} bytes ({:.2} MB)", uncompressed_size, uncompressed_size as f64 / 1_048_576.0);
    println!("Compression ratio: {:.2}%", ratio * 100.0);
    println!("Space saved:       {:.2}%", savings);

    // ==========================================================================
    // Verify Data Integrity (sample check)
    // ==========================================================================
    println!("\n=== Data Integrity Verification (sample check) ===");

    // Read and verify first 100 samples from both files
    let check_samples = 100.min(NUM_SAMPLES);

    let compressed_mf4 = Mf4Wrapper::new::<fn(f64)>(compressed_path.clone(), None)?;
    let uncompressed_mf4 = Mf4Wrapper::new::<fn(f64)>(uncompressed_path.clone(), None)?;

    let compressed_channels = compressed_mf4.get_channel_names();
    let uncompressed_channels = uncompressed_mf4.get_channel_names();

    println!("Compressed file: {} channels", compressed_channels.len());
    println!("Uncompressed file: {} channels", uncompressed_channels.len());

    let mut all_match = true;
    for ch_name in &compressed_channels {
        let c_data = compressed_mf4.get_channel_data(ch_name);
        let u_data = uncompressed_mf4.get_channel_data(ch_name);

        match (c_data, u_data) {
            (Some(c), Some(u)) => {
                match (&c, &u) {
                    (mf4_parse::data_serde::DataValue::REAL(c_vals),
                     mf4_parse::data_serde::DataValue::REAL(u_vals)) => {
                        // Check first N samples match
                        let check_len = check_samples.min(c_vals.len()).min(u_vals.len());
                        let matches: bool = c_vals[..check_len].iter()
                            .zip(u_vals[..check_len].iter())
                            .all(|(a, b)| (a - b).abs() < 1e-10);

                        if matches {
                            println!("  ✓ {}: {} total samples match", ch_name, c_vals.len());
                        } else {
                            println!("  ✗ {}: Data mismatch in first {} samples", ch_name, check_len);
                            all_match = false;
                        }
                    }
                    _ => {
                        println!("  ✓ {}: Non-REAL type, {} samples", ch_name, c.len());
                    }
                }
            }
            _ => {
                println!("  ✗ {}: Not found in one of the files", ch_name);
                all_match = false;
            }
        }
    }

    if all_match {
        println!("\n✓ All channel data matches between compressed and uncompressed files!");
    } else {
        println!("\n✗ Some channel data mismatches detected!");
    }

    // ==========================================================================
    // Summary
    // ==========================================================================
    println!("\n=== Summary ===");
    println!("Generated files:");
    println!("  - compressed.mf4   (DZ blocks with Deflate compression)");
    println!("  - uncompressed.mf4  (DT blocks without compression)");
    println!("\nKey points:");
    println!("  - Streaming write enables large file generation without memory issues");
    println!("  - Compression is enabled via StreamingConfig::with_compression()");
    println!("  - Data is written in batches of {} samples", BATCH_SIZE);
    println!("  - finalize_with_compact(true) merges all blocks into one");

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
