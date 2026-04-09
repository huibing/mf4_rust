//! Example: Generate MF4 file with one Data Group per Channel Group
//!
//! This example creates the same data as time_series_demo but uses a separate
//! data group for each channel group. This structure is more compatible with
//! MDA tools that rely on a single, uninterleaved data block.
//!
//! Run with: cargo run --example time_series_one_dg_per_cg --features write
//!
//! Output: test/time_series_demo_separate_dg.mf4

use std::path::PathBuf;

use mf4_parse::writer::{
    BusType, ChannelBuilder, ChannelGroupBuilder, DataGroupBuilder, Mf4Builder, Mf4Metadata,
    SourceInfoBuilder, SourceType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Time Series Demo (One Data Group per Channel Group) ===\n");

    let metadata = Mf4Metadata::new()
        .with_author("MF4 Parse Demo")
        .with_organization("Test Organization")
        .with_project("Time Series Demo (one DG per CG)")
        .with_comment("Same data as time_series_demo but each CG in its own DG");

    let mut builder = Mf4Builder::new(metadata);

    // ==========================================================================
    // Channel Group 1: Fast sampling (100Hz, 10ms period)
    // ==========================================================================
    let source_fast = SourceInfoBuilder::new()
        .name("ADC_100Hz")
        .path("DAQ/Card1/Channel1")
        .source_type(SourceType::Io)
        .comment("High-speed analog input card, 100Hz sampling rate")
        .build()?;

    let cg_fast = ChannelGroupBuilder::new()
        .name("FastSampling_100Hz")
        .acq_source(source_fast)
        .master(ChannelBuilder::new_master_time("time_fast"))
        .channel(
            ChannelBuilder::new("SineWave")
                .data_type(4)
                .unit("V")
                .comment("1Hz sine wave")
                .build()?,
        )
        .channel(
            ChannelBuilder::new("CosineWave")
                .data_type(4)
                .unit("V")
                .comment("1Hz cosine wave")
                .build()?,
        )
        .build()?;

    builder.add_data_group(DataGroupBuilder::new().channel_group(cg_fast).build()?);

    // ==========================================================================
    // Channel Group 2: Medium sampling (20Hz, 50ms period)
    // ==========================================================================
    let source_medium = SourceInfoBuilder::new()
        .name("CAN_Bus_20Hz")
        .path("CAN1")
        .source_type(SourceType::Bus)
        .bus_type(BusType::Can)
        .comment("CAN bus signal acquisition, 20Hz sampling rate")
        .build()?;

    let cg_medium = ChannelGroupBuilder::new()
        .name("MediumSampling_20Hz")
        .acq_source(source_medium)
        .master(ChannelBuilder::new_master_time("time_medium"))
        .channel(
            ChannelBuilder::new("SquareWave")
                .data_type(4)
                .unit("V")
                .comment("0.5Hz square wave")
                .build()?,
        )
        .channel(
            ChannelBuilder::new("Counter")
                .data_type(0)
                .bit_count(8)
                .unit("count")
                .comment("0-255 counter")
                .build()?,
        )
        .build()?;

    builder.add_data_group(DataGroupBuilder::new().channel_group(cg_medium).build()?);

    // ==========================================================================
    // Channel Group 3: Slow sampling (10Hz, 100ms period)
    // ==========================================================================
    let source_slow = SourceInfoBuilder::new()
        .name("ECU_Monitor_10Hz")
        .path("ECU/Internal")
        .source_type(SourceType::Ecu)
        .comment("ECU internal monitoring, 10Hz sampling rate")
        .build()?;

    let cg_slow = ChannelGroupBuilder::new()
        .name("SlowSampling_10Hz")
        .acq_source(source_slow)
        .master(ChannelBuilder::new_master_time("time_slow"))
        .channel(
            ChannelBuilder::new("RandomNoise")
                .data_type(4)
                .unit("mV")
                .comment("Random noise signal")
                .build()?,
        )
        .channel(
            ChannelBuilder::new("Status")
                .data_type(0)
                .bit_count(8)
                .comment("System status")
                .build()?,
        )
        .build()?;

    builder.add_data_group(DataGroupBuilder::new().channel_group(cg_slow).build()?);

    // ==========================================================================
    // Generate data (100 seconds)
    // ==========================================================================
    let duration_sec = 100.0_f64;

    // Fast: 100 Hz
    let n_fast = (duration_sec / 0.01) as usize;
    let time_fast: Vec<f64> = (0..n_fast).map(|i| i as f64 * 0.01).collect();
    let sine_data: Vec<f64> = time_fast
        .iter()
        .map(|t| (2.0 * std::f64::consts::PI * t).sin())
        .collect();
    let cosine_data: Vec<f64> = time_fast
        .iter()
        .map(|t| (2.0 * std::f64::consts::PI * t).cos())
        .collect();

    // Medium: 20 Hz
    let n_medium = (duration_sec / 0.05) as usize;
    let time_medium: Vec<f64> = (0..n_medium).map(|i| i as f64 * 0.05).collect();
    let square_data: Vec<f64> = time_medium
        .iter()
        .map(|t| {
            if (*t * 0.5).floor() % 2.0 == 0.0 {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    let counter_data: Vec<u8> = (0..n_medium).map(|i| (i % 256) as u8).collect();

    // Slow: 10 Hz
    let n_slow = (duration_sec / 0.1) as usize;
    let time_slow: Vec<f64> = (0..n_slow).map(|i| i as f64 * 0.1).collect();
    let random_data: Vec<f64> = {
        let mut state: u64 = 12345;
        (0..n_slow)
            .map(|_| {
                state = state.wrapping_mul(1103515245).wrapping_add(12345);
                (state % 1000) as f64 / 100.0 - 5.0
            })
            .collect()
    };
    let status_data: Vec<u8> = time_slow
        .iter()
        .map(|t| (*t / 2.0).floor() as u8 % 4)
        .collect();

    // ==========================================================================
    // Set channel data
    // ==========================================================================
    builder.set_channel_data("time_fast", &time_fast)?;
    builder.set_channel_data("SineWave", &sine_data)?;
    builder.set_channel_data("CosineWave", &cosine_data)?;

    builder.set_channel_data("time_medium", &time_medium)?;
    builder.set_channel_data("SquareWave", &square_data)?;
    builder.set_channel_data("Counter", &counter_data)?;

    builder.set_channel_data("time_slow", &time_slow)?;
    builder.set_channel_data("RandomNoise", &random_data)?;
    builder.set_channel_data("Status", &status_data)?;

    // ==========================================================================
    // Write
    // ==========================================================================
    let output_path = PathBuf::from("test/time_series_demo_separate_dg.mf4");
    println!("Writing MF4 file (one DG per CG) to: {}", output_path.display());
    println!("  - Fast (100Hz): {} samples", n_fast);
    println!("  - Medium (20Hz): {} samples", n_medium);
    println!("  - Slow (10Hz): {} samples", n_slow);

    builder.write(output_path.clone())?;

    println!("Successfully wrote: {}", output_path.display());
    Ok(())
}
