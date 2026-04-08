use mf4_parse::Mf4Wrapper;
use mf4_parse::ChannelLink;
use std::path::PathBuf;


pub fn display_channel_info(channel_name: &str, mf4: &Mf4Wrapper) {
    if let Some(ChannelLink(cn, cg, _)) = mf4.get_channel_link(channel_name) {
        println!("channel info: \n{}", cn);
        println!("channel group comment: {:?}", cg.get_comment());
        println!("channel group source: {:?}", cg.get_acq_name());
        println!("channel group source info: {}", cg.get_acq_source());
        if let Some(ar) = cn.get_array() {
            println!("channel array info: {:?}", ar);
            println!("channel array names {:?}", ar.generate_array_names(cn.get_name()));
            println!("channel array indexes {:?}", ar.generate_array_indexs());
        }
        if let Some(chs) = cn.get_sub_channels() {
            println!("channel subchannels info :");
            for ch in chs {
                println!("channel subchannel info: {}", ch);
            }
        }
    } else {
        println!("no channel info found for {}", channel_name);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example: Read MF4 file and display channel info
    let mf4: Mf4Wrapper = Mf4Wrapper::new::<fn(f64)>(PathBuf::from("test/demo.mf4"), None)?;

    println!("Header time stamp: {:?}", mf4.get_time_stamp());

    let channel_names = mf4.get_channel_names();
    println!("Total channels: {}", channel_names.len());

    // Display info for specific channels
    display_channel_info("Nested_structures", &mf4);
    display_channel_info("Channel_lookup_with_default_axis", &mf4);

    // Get channel data
    if let Some(channel_name) = channel_names.first() {
        if let Some(data) = mf4.get_channel_data(channel_name) {
            println!("First channel '{}' data: {}", channel_name, data);
        }
    }

    // Run write example if feature is enabled
    #[cfg(feature = "write")]
    {
        println!("\n=== Running Write Example ===");
        write_time_series_example()?;
    }

    // Run one-DG-per-CG write example if feature is enabled
    #[cfg(feature = "write")]
    {
        println!("\n=== Running Write Example (one DG per CG) ===");
        write_time_series_one_dg_per_cg()?;
    }

    // Run streaming write example if feature is enabled
    #[cfg(feature = "streaming")]
    {
        println!("\n=== Running Streaming Write Example ===");
        write_streaming_example()?;
    }

    Ok(())
}

/// Write example: Generate time series data with different periods and save to MF4
#[cfg(feature = "write")]
fn write_time_series_example() -> Result<(), Box<dyn std::error::Error>> {
    use mf4_parse::writer::{
        Mf4Builder, Mf4Metadata, ChannelBuilder, ChannelGroupBuilder,
        DataGroupBuilder, ConversionBuilder, SourceInfoBuilder, SourceType, BusType,
    };
    use mf4_parse::writer::builder::ConversionParams;

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
        .data_type(4)      // FLOAT64 LE
        .unit("V")
        .comment("1Hz sine wave signal")
        .build()?;
    let cosine_wave = ChannelBuilder::new("CosineWave")
        .data_type(4)      // FLOAT64 LE
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
        .data_type(4)      // FLOAT64 LE
        .unit("V")
        .comment("0.5Hz square wave signal")
        .build()?;
    let counter = ChannelBuilder::new("Counter")
        .data_type(0)      // UINT8
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
        .data_type(4)      // FLOAT64 LE
        .unit("mV")
        .comment("Random noise signal")
        .build()?;
    let status = ChannelBuilder::new("Status")
        .data_type(0)      // UINT8
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
    let sine_data: Vec<f64> = time_fast_data.iter().map(|t| (2.0 * std::f64::consts::PI * t).sin()).collect();
    let cosine_data: Vec<f64> = time_fast_data.iter().map(|t| (2.0 * std::f64::consts::PI * t).cos()).collect();

    // Medium sampling: 50ms period = 20 samples/sec
    let period_medium = 0.05;
    let n_medium = (duration_sec / period_medium) as usize;
    let time_medium_data: Vec<f64> = (0..n_medium).map(|i| i as f64 * period_medium).collect();
    // Square wave: 0.5Hz, toggles every 1 second
    let square_data: Vec<f64> = time_medium_data.iter().map(|t| if (*t * 0.5).floor() % 2.0 == 0.0 { 1.0 } else { 0.0 }).collect();
    // Counter: 0-255, wrapping
    let counter_data: Vec<u8> = (0..n_medium).map(|i| (i % 256) as u8).collect();

    // Slow sampling: 100ms period = 10 samples/sec
    let period_slow = 0.1;
    let n_slow = (duration_sec / period_slow) as usize;
    let time_slow_data: Vec<f64> = (0..n_slow).map(|i| i as f64 * period_slow).collect();
    // Random noise: simple pseudo-random using linear congruential generator
    let random_data: Vec<f64> = {
        let mut state: u64 = 12345;
        (0..n_slow).map(|_| {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            (state % 1000) as f64 / 100.0 - 5.0  // Range: -5.0 to 5.0
        }).collect()
    };
    // Status: cycle through 0,1,2,3 every 2 seconds
    let status_data: Vec<u8> = time_slow_data.iter().map(|t| (*t / 2.0).floor() as u8 % 4).collect();

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

/// One-shot write example: same data as write_time_series_example but each channel
/// group gets its own dedicated data group (one CG per DG).  This matches the
/// structure produced by the streaming writer and is more compatible with MDA tools
/// that rely on a single, uninterleaved data block when resolving master time channels.
#[cfg(feature = "write")]
fn write_time_series_one_dg_per_cg() -> Result<(), Box<dyn std::error::Error>> {
    use mf4_parse::writer::{
        Mf4Builder, Mf4Metadata, ChannelBuilder, ChannelGroupBuilder,
        DataGroupBuilder, SourceInfoBuilder, SourceType, BusType,
    };

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
        .channel(ChannelBuilder::new("SineWave").data_type(4).unit("V").comment("1Hz sine wave").build()?)
        .channel(ChannelBuilder::new("CosineWave").data_type(4).unit("V").comment("1Hz cosine wave").build()?)
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
        .channel(ChannelBuilder::new("SquareWave").data_type(4).unit("V").comment("0.5Hz square wave").build()?)
        .channel(ChannelBuilder::new("Counter").data_type(0).bit_count(8).unit("count").comment("0-255 counter").build()?)
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
        .channel(ChannelBuilder::new("RandomNoise").data_type(4).unit("mV").comment("Random noise signal").build()?)
        .channel(ChannelBuilder::new("Status").data_type(0).bit_count(8).comment("System status").build()?)
        .build()?;

    builder.add_data_group(DataGroupBuilder::new().channel_group(cg_slow).build()?);

    // ==========================================================================
    // Generate data (10 seconds)
    // ==========================================================================
    let duration_sec = 10.0_f64;

    // Fast: 100 Hz
    let n_fast = (duration_sec / 0.01) as usize;
    let time_fast: Vec<f64> = (0..n_fast).map(|i| i as f64 * 0.01).collect();
    let sine_data: Vec<f64> = time_fast.iter().map(|t| (2.0 * std::f64::consts::PI * t).sin()).collect();
    let cosine_data: Vec<f64> = time_fast.iter().map(|t| (2.0 * std::f64::consts::PI * t).cos()).collect();

    // Medium: 20 Hz
    let n_medium = (duration_sec / 0.05) as usize;
    let time_medium: Vec<f64> = (0..n_medium).map(|i| i as f64 * 0.05).collect();
    let square_data: Vec<f64> = time_medium.iter().map(|t| if (*t * 0.5).floor() % 2.0 == 0.0 { 1.0 } else { 0.0 }).collect();
    let counter_data: Vec<u8> = (0..n_medium).map(|i| (i % 256) as u8).collect();

    // Slow: 10 Hz
    let n_slow = (duration_sec / 0.1) as usize;
    let time_slow: Vec<f64> = (0..n_slow).map(|i| i as f64 * 0.1).collect();
    let random_data: Vec<f64> = {
        let mut state: u64 = 12345;
        (0..n_slow).map(|_| {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            (state % 1000) as f64 / 100.0 - 5.0
        }).collect()
    };
    let status_data: Vec<u8> = time_slow.iter().map(|t| (*t / 2.0).floor() as u8 % 4).collect();

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

/// Streaming write example: Simulate real-time data acquisition where data is written
/// incrementally every second. The number of samples per write depends on the sampling rate.
#[cfg(feature = "streaming")]
fn write_streaming_example() -> Result<(), Box<dyn std::error::Error>> {
    use mf4_parse::writer::{
        Mf4StreamWriter, Mf4Metadata, StreamingConfig, ChannelDef,
        SourceInfoBuilder, SourceType, BusType,
    };
    use mf4_parse::writer::stream_writer::{ChannelGroupDefBuilder, StreamingDataGroup};
    use std::time::{Instant, Duration};

    // Create metadata
    let metadata = Mf4Metadata::new()
        .with_author("MF4 Parse Demo")
        .with_organization("Test Organization")
        .with_project("Streaming Time Series Demo")
        .with_comment("Streaming data acquisition with incremental writes");

    // Create streaming configuration
    let config = StreamingConfig::new()
        .with_block_size(100_000); // 100 KB blocks

    let mut writer = Mf4StreamWriter::with_config(
        PathBuf::from("test/streaming_demo.mf4"),
        metadata,
        config,
    )?;

    // ==========================================================================
    // Define Channel Group 1: Fast sampling (100Hz = 100 samples/sec)
    // ==========================================================================
    let time_fast = ChannelDef::new_master("time_fast");
    let sine_wave = ChannelDef::new("SineWave")
        .data_type(4)
        .unit("V");
    let cosine_wave = ChannelDef::new("CosineWave")
        .data_type(4)
        .unit("V");

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
    let square_wave = ChannelDef::new("SquareWave")
        .data_type(4)
        .unit("V");
    let counter = ChannelDef::new("Counter")
        .data_type(0)
        .bit_count(8)
        .unit("count");

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
    let random_noise = ChannelDef::new("RandomNoise")
        .data_type(4)
        .unit("mV");
    let status = ChannelDef::new("Status")
        .data_type(0)
        .bit_count(8);

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
    println!("Structure finalized, ready for streaming data...");

    // ==========================================================================
    // Streaming write: 10 seconds of data, writing once per second
    // ==========================================================================
    let duration_sec = 10;
    let start_time = Instant::now();

    // Sampling rates (samples per second)
    let samples_per_sec_fast = 100;    // 100Hz
    let samples_per_sec_medium = 20;   // 20Hz
    let samples_per_sec_slow = 10;     // 10Hz

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
            let square_val = if (t * 0.5).floor() % 2.0 == 0.0 { 1.0 } else { 0.0 };
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


#[cfg(test)]
pub mod test {
    use rstest::*;
    use mf4_parse::parser::Mf4Wrapper;
    use std::path::PathBuf;
    use super::display_channel_info;

    #[rstest]
    fn mf4_wrapper_test() {
        let mf4: Mf4Wrapper = Mf4Wrapper::new::<fn(f64)>(PathBuf::from("test/demo.mf4"), None).unwrap();
        let channel_names: Vec<String> = mf4.get_channel_names();
        println!("{:?}", channel_names);
        display_channel_info("Nested_structures", &mf4);
        display_channel_info("Channel_lookup_with_default_axis", &mf4);
        let new: Mf4Wrapper = Mf4Wrapper::new::<fn(f64)>(PathBuf::from("test/string_and_array.mf4"), None).unwrap();
        display_channel_info("Channel_lookup_with_default_axis[0][0][2]", &new);
        let d: mf4_parse::data_serde::DataValue = new.get_channel_data("Channel_lookup_with_default_axis[0][0][2]").unwrap();
        println!("{:?}\n value ends\n", d);
    }
}
