//! Example: Streaming write for real-time data acquisition
//!
//! This example demonstrates how to write MF4 data incrementally, simulating
//! real-time data acquisition where data is written every second. Each channel
//! group has its own data group for better compatibility with MDF4 readers.
//!
//! Run with: cargo run --example streaming_demo --features streaming
//!
//! Output: test/streaming_demo.mf4

use std::path::PathBuf;
use std::time::{Duration, Instant};

use mf4_parse::writer::{
    BusType, ChannelDef, Mf4Metadata, Mf4StreamWriter, SourceInfoBuilder, SourceType,
    StreamingConfig,
};
use mf4_parse::writer::stream_writer::{ChannelGroupDefBuilder, StreamingDataGroup};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Streaming Write Demo ===\n");

    // Create metadata
    let metadata = Mf4Metadata::new()
        .with_author("MF4 Parse Demo")
        .with_organization("Test Organization")
        .with_project("Streaming Time Series Demo")
        .with_comment("Streaming data acquisition with incremental writes");

    // Create streaming configuration
    let config = StreamingConfig::new().with_block_size(100_000); // 100 KB blocks

    let mut writer = Mf4StreamWriter::with_config(
        PathBuf::from("test/streaming_demo.mf4"),
        metadata,
        config,
    )?;

    // ==========================================================================
    // Define Channel Group 1: Fast sampling (100Hz = 100 samples/sec)
    // ==========================================================================
    let time_fast = ChannelDef::new_master("time_fast");
    let sine_wave = ChannelDef::new("SineWave").data_type(4).unit("V");
    let cosine_wave = ChannelDef::new("CosineWave").data_type(4).unit("V");

    // Source info for fast sampling channel group
    let source_fast = SourceInfoBuilder::new()
        .name("ADC_100Hz")
        .path("DAQ/Card1/Channel1")
        .source_type(SourceType::Io)
        .comment("High-speed analog input card, 100Hz sampling rate, 10ms period")
        .build()?;

    let cg_fast = ChannelGroupDefBuilder::new()
        .name("FastSampling_100Hz")
        .acq_source(source_fast)
        .master(time_fast)
        .channel(sine_wave)
        .channel(cosine_wave)
        .build()?;

    // ==========================================================================
    // Define Channel Group 2: Medium sampling (20Hz = 20 samples/sec)
    // ==========================================================================
    let time_medium = ChannelDef::new_master("time_medium");
    let square_wave = ChannelDef::new("SquareWave").data_type(4).unit("V");
    let counter = ChannelDef::new("Counter").data_type(0).bit_count(8).unit("count");

    // Source info for medium sampling channel group (CAN bus)
    let source_medium = SourceInfoBuilder::new()
        .name("CAN_Bus_20Hz")
        .path("CAN1")
        .source_type(SourceType::Bus)
        .bus_type(BusType::Can)
        .comment("CAN bus signal acquisition, 20Hz sampling rate, 50ms period")
        .build()?;

    let cg_medium = ChannelGroupDefBuilder::new()
        .name("MediumSampling_20Hz")
        .acq_source(source_medium)
        .master(time_medium)
        .channel(square_wave)
        .channel(counter)
        .build()?;

    // ==========================================================================
    // Define Channel Group 3: Slow sampling (10Hz = 10 samples/sec)
    // ==========================================================================
    let time_slow = ChannelDef::new_master("time_slow");
    let random_noise = ChannelDef::new("RandomNoise").data_type(4).unit("mV");
    let status = ChannelDef::new("Status").data_type(0).bit_count(8);

    // Source info for slow sampling channel group (ECU)
    let source_slow = SourceInfoBuilder::new()
        .name("ECU_Monitor_10Hz")
        .path("ECU/Internal")
        .source_type(SourceType::Ecu)
        .comment("ECU internal monitoring, 10Hz sampling rate, 100ms period")
        .build()?;

    let cg_slow = ChannelGroupDefBuilder::new()
        .name("SlowSampling_10Hz")
        .acq_source(source_slow)
        .master(time_slow)
        .channel(random_noise)
        .channel(status)
        .build()?;

    // Add each channel group as its own data group.
    // Using separate DGs (one per channel group) is the recommended approach when
    // channel groups have different sampling rates. MDF4 readers handle single-CG
    // DGs more reliably than multi-CG DGs.
    writer.add_data_group(StreamingDataGroup::new(cg_fast)?)?;
    writer.add_data_group(StreamingDataGroup::new(cg_medium)?)?;
    writer.add_data_group(StreamingDataGroup::new(cg_slow)?)?;

    // Finalize structure (write file header blocks)
    writer.finalize_structure()?;
    println!("Structure finalized, ready for streaming data...\n");

    // ==========================================================================
    // Streaming write: 10 seconds of data, writing once per second
    // ==========================================================================
    let duration_sec = 10;
    let start_time = Instant::now();

    // Sampling rates (samples per second)
    let samples_per_sec_fast = 100; // 100Hz
    let samples_per_sec_medium = 20; // 20Hz
    let samples_per_sec_slow = 10; // 10Hz

    println!("Starting streaming write: {} seconds, writing every second", duration_sec);
    println!("  - Fast (100Hz): {} samples/sec", samples_per_sec_fast);
    println!("  - Medium (20Hz): {} samples/sec", samples_per_sec_medium);
    println!("  - Slow (10Hz): {} samples/sec", samples_per_sec_slow);

    // Pseudo-random state for noise generation
    let mut noise_state: u64 = 12345;

    for sec in 0..duration_sec {
        let loop_start = Instant::now();

        // --- Fast sampling (100Hz): write 100 samples ---
        for i in 0..samples_per_sec_fast {
            let sample_idx = sec * samples_per_sec_fast + i;
            let t = sample_idx as f64 * 0.01; // 10ms period

            writer.start_record(0, 0)?; // dg_index=0 (fast DG), cg_index=0
            writer.set_channel_value("time_fast", t)?;
            writer.set_channel_value("SineWave", (2.0 * std::f64::consts::PI * t).sin())?;
            writer.set_channel_value("CosineWave", (2.0 * std::f64::consts::PI * t).cos())?;
            writer.flush_record()?;
        }

        // --- Medium sampling (20Hz): write 20 samples ---
        for i in 0..samples_per_sec_medium {
            let sample_idx = sec * samples_per_sec_medium + i;
            let t = sample_idx as f64 * 0.05; // 50ms period

            writer.start_record(1, 0)?; // dg_index=1 (medium DG), cg_index=0
            writer.set_channel_value("time_medium", t)?;
            // Square wave: toggles every 1 second
            let square_val = if (t * 0.5).floor() % 2.0 == 0.0 {
                1.0
            } else {
                0.0
            };
            writer.set_channel_value("SquareWave", square_val)?;
            writer.set_channel_value("Counter", (sample_idx % 256) as u8)?;
            writer.flush_record()?;
        }

        // --- Slow sampling (10Hz): write 10 samples ---
        for i in 0..samples_per_sec_slow {
            let sample_idx = sec * samples_per_sec_slow + i;
            let t = sample_idx as f64 * 0.1; // 100ms period

            writer.start_record(2, 0)?; // dg_index=2 (slow DG), cg_index=0
            writer.set_channel_value("time_slow", t)?;

            // Generate pseudo-random noise
            noise_state = noise_state.wrapping_mul(1103515245).wrapping_add(12345);
            let noise_val = ((noise_state % 1000) as f64 / 100.0) - 5.0;
            writer.set_channel_value("RandomNoise", noise_val)?;

            // Status: cycle through 0,1,2,3 every 2 seconds
            let status_val = (t / 2.0).floor() as u8 % 4;
            writer.set_channel_value("Status", status_val)?;
            writer.flush_record()?;
        }

        // Calculate elapsed time and sleep to simulate 1-second interval
        let elapsed = loop_start.elapsed();
        let target_duration = Duration::from_secs(1);
        if elapsed < target_duration {
            std::thread::sleep(target_duration - elapsed);
        }

        let total_elapsed = start_time.elapsed();
        println!(
            "  Second {:2}: wrote {} fast + {} medium + {} slow samples (elapsed: {:.2}s)",
            sec + 1,
            samples_per_sec_fast,
            samples_per_sec_medium,
            samples_per_sec_slow,
            total_elapsed.as_secs_f64()
        );
    }

    // Finalize the file (write data blocks and update metadata)
    writer.finalize_with_compact(true)?;

    let total_time = start_time.elapsed();
    let total_records = writer.total_records();

    println!("\nStreaming write completed:");
    println!("  - Total time: {:.2}s", total_time.as_secs_f64());
    println!("  - Total records: {}", total_records);
    println!("  - Output file: test/streaming_demo.mf4");

    Ok(())
}
