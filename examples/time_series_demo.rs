//! Example: Generate MF4 file with time series data at different sampling rates
//!
//! This example demonstrates how to create an MF4 file with multiple channel groups,
//! each having different sampling rates. All channel groups are stored in a single
//! data group with record IDs for interleaving.
//!
//! Run with: cargo run --example time_series_demo --features write
//!
//! Output: test/time_series_demo.mf4

use std::path::PathBuf;

use mf4_parse::writer::{
    BusType, ChannelBuilder, ChannelGroupBuilder, ConversionBuilder, DataGroupBuilder,
    Mf4Builder, Mf4Metadata, SourceInfoBuilder, SourceType,
};
use mf4_parse::writer::builder::ConversionParams;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Time Series Demo (Single Data Group with Multiple Channel Groups) ===\n");

    // Create metadata
    let metadata = Mf4Metadata::new()
        .with_author("MF4 Parse Demo")
        .with_organization("Test Organization")
        .with_project("Time Series Demo")
        .with_comment("Generated time series data with multiple sampling rates");

    let mut builder = Mf4Builder::new(metadata);

    // Define the enum conversion (Value2Text for Status signal)
    let status_conversion = ConversionBuilder {
        name: None,
        cc_type: 7, // Value to text
        params: ConversionParams::Value2Text {
            keys: vec![0.0, 1.0, 2.0, 3.0],
            texts: vec![
                "Normal".to_string(),
                "Not Good".to_string(),
                "Error".to_string(),
                "Emergency".to_string(),
            ],
            default: "Unknown".to_string(),
        },
        unit: None,
        comment: Some("System status enumeration".to_string()),
    };

    // ==========================================================================
    // Channel Group 1: Fast sampling (10ms period = 100Hz)
    // Signals: Sine wave, Cosine wave
    // ==========================================================================
    let time_fast = ChannelBuilder::new_master_time("time_fast");
    let sine_wave = ChannelBuilder::new("SineWave")
        .data_type(4) // FLOAT64 LE
        .unit("V")
        .comment("1Hz sine wave signal")
        .build()?;
    let cosine_wave = ChannelBuilder::new("CosineWave")
        .data_type(4) // FLOAT64 LE
        .unit("V")
        .comment("1Hz cosine wave signal")
        .build()?;

    // Source info for fast sampling channel group
    let source_fast = SourceInfoBuilder::new()
        .name("ADC_100Hz")
        .path("DAQ/Card1/Channel1")
        .source_type(SourceType::Io)
        .comment("High-speed analog input card, 100Hz sampling rate, 10ms period")
        .build()?;

    let cg_fast = ChannelGroupBuilder::new()
        .name("FastSampling_100Hz")
        .acq_source(source_fast)
        .master(time_fast)
        .channel(sine_wave)
        .channel(cosine_wave)
        .build()?;

    // ==========================================================================
    // Channel Group 2: Medium sampling (50ms period = 20Hz)
    // Signals: Square wave, Counter
    // ==========================================================================
    let time_medium = ChannelBuilder::new_master_time("time_medium");
    let square_wave = ChannelBuilder::new("SquareWave")
        .data_type(4) // FLOAT64 LE
        .unit("V")
        .comment("0.5Hz square wave signal")
        .build()?;
    let counter = ChannelBuilder::new("Counter")
        .data_type(0) // UINT8
        .bit_count(8)
        .unit("count")
        .comment("0-255 periodic counter")
        .build()?;

    // Source info for medium sampling channel group (CAN bus)
    let source_medium = SourceInfoBuilder::new()
        .name("CAN_Bus_20Hz")
        .path("CAN1")
        .source_type(SourceType::Bus)
        .bus_type(BusType::Can)
        .comment("CAN bus signal acquisition, 20Hz sampling rate, 50ms period")
        .build()?;

    let cg_medium = ChannelGroupBuilder::new()
        .name("MediumSampling_20Hz")
        .acq_source(source_medium)
        .master(time_medium)
        .channel(square_wave)
        .channel(counter)
        .build()?;

    // ==========================================================================
    // Channel Group 3: Slow sampling (100ms period = 10Hz)
    // Signals: Random noise, Status enum
    // ==========================================================================
    let time_slow = ChannelBuilder::new_master_time("time_slow");
    let random_noise = ChannelBuilder::new("RandomNoise")
        .data_type(4) // FLOAT64 LE
        .unit("mV")
        .comment("Random noise signal")
        .build()?;
    let status = ChannelBuilder::new("Status")
        .data_type(0) // UINT8
        .bit_count(8)
        .comment("System status enum: 0=Normal, 1=Not Good, 2=Error, 3=Emergency")
        .conversion(status_conversion)
        .build()?;

    // Source info for slow sampling channel group (ECU)
    let source_slow = SourceInfoBuilder::new()
        .name("ECU_Monitor_10Hz")
        .path("ECU/Internal")
        .source_type(SourceType::Ecu)
        .comment("ECU internal monitoring, 10Hz sampling rate, 100ms period")
        .build()?;

    let cg_slow = ChannelGroupBuilder::new()
        .name("SlowSampling_10Hz")
        .acq_source(source_slow)
        .master(time_slow)
        .channel(random_noise)
        .channel(status)
        .build()?;

    // Create data group with all three channel groups
    let dg = DataGroupBuilder::new()
        .channel_group(cg_fast)
        .channel_group(cg_medium)
        .channel_group(cg_slow)
        .build()?;

    builder.add_data_group(dg);

    // ==========================================================================
    // Generate time series data (10 seconds duration)
    // ==========================================================================
    let duration_sec = 10.0;

    // Fast sampling: 10ms period = 100 samples/sec
    let period_fast = 0.01;
    let n_fast = (duration_sec / period_fast) as usize;
    let time_fast_data: Vec<f64> = (0..n_fast).map(|i| i as f64 * period_fast).collect();
    let sine_data: Vec<f64> = time_fast_data
        .iter()
        .map(|t| (2.0 * std::f64::consts::PI * t).sin())
        .collect();
    let cosine_data: Vec<f64> = time_fast_data
        .iter()
        .map(|t| (2.0 * std::f64::consts::PI * t).cos())
        .collect();

    // Medium sampling: 50ms period = 20 samples/sec
    let period_medium = 0.05;
    let n_medium = (duration_sec / period_medium) as usize;
    let time_medium_data: Vec<f64> = (0..n_medium)
        .map(|i| i as f64 * period_medium)
        .collect();
    // Square wave: 0.5Hz, toggles every 1 second
    let square_data: Vec<f64> = time_medium_data
        .iter()
        .map(|t| {
            if (*t * 0.5).floor() % 2.0 == 0.0 {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    // Counter: 0-255, wrapping
    let counter_data: Vec<u8> = (0..n_medium).map(|i| (i % 256) as u8).collect();

    // Slow sampling: 100ms period = 10 samples/sec
    let period_slow = 0.1;
    let n_slow = (duration_sec / period_slow) as usize;
    let time_slow_data: Vec<f64> = (0..n_slow).map(|i| i as f64 * period_slow).collect();
    // Random noise: simple pseudo-random using linear congruential generator
    let random_data: Vec<f64> = {
        let mut state: u64 = 12345;
        (0..n_slow)
            .map(|_| {
                state = state.wrapping_mul(1103515245).wrapping_add(12345);
                (state % 1000) as f64 / 100.0 - 5.0 // Range: -5.0 to 5.0
            })
            .collect()
    };
    // Status: cycle through 0,1,2,3 every 2 seconds
    let status_data: Vec<u8> = time_slow_data
        .iter()
        .map(|t| (*t / 2.0).floor() as u8 % 4)
        .collect();

    // ==========================================================================
    // Set channel data
    // ==========================================================================
    builder.set_channel_data("time_fast", &time_fast_data)?;
    builder.set_channel_data("SineWave", &sine_data)?;
    builder.set_channel_data("CosineWave", &cosine_data)?;

    builder.set_channel_data("time_medium", &time_medium_data)?;
    builder.set_channel_data("SquareWave", &square_data)?;
    builder.set_channel_data("Counter", &counter_data)?;

    builder.set_channel_data("time_slow", &time_slow_data)?;
    builder.set_channel_data("RandomNoise", &random_data)?;
    builder.set_channel_data("Status", &status_data)?;

    // ==========================================================================
    // Write to file
    // ==========================================================================
    let output_path = PathBuf::from("test/time_series_demo.mf4");
    println!("Writing MF4 file to: {}", output_path.display());
    println!("  - Fast sampling (100Hz): {} samples", n_fast);
    println!("  - Medium sampling (20Hz): {} samples", n_medium);
    println!("  - Slow sampling (10Hz): {} samples", n_slow);

    builder.write(output_path.clone())?;

    println!("Successfully wrote MF4 file: {}", output_path.display());
    println!("Note: File is not deleted. You can verify it using MF4 reader tools.");

    Ok(())
}
