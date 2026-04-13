//! Example: Streaming write with compression for real-time data acquisition
//!
//! Simulates 60 seconds of multi-rate automotive data acquisition using the
//! streaming writer with Deflate compression. The three channel groups produce
//! enough data to fill multiple DZ blocks (the MDF4 protocol caps each DZ block
//! at 4 MB, so the fast group generates 3 DZ blocks).
//!
//! Write time is measured separately from any sleep time so the reported
//! throughput reflects pure MF4 I/O performance.
//!
//! Run with:
//!   cargo run --release --example streaming_demo --features "streaming,compression"
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
    println!("=== Streaming Write Demo (with compression) ===\n");

    // ─── Simulation parameters ────────────────────────────────────────────────
    const SIM_SECONDS: usize = 60;   // simulated recording duration
    const FAST_HZ:    usize = 5_000; // 5 kHz — engine / powertrain
    const MEDIUM_HZ:  usize =   500; // 500 Hz — chassis / brakes
    const SLOW_HZ:    usize =   100; // 100 Hz — thermal / fuel

    // Record sizes: time (f64) + 3 signals (f64) = 4 × 8 = 32 bytes per record
    let fast_bytes   = FAST_HZ   * SIM_SECONDS * 32; // ~9.6 MB → 3 DZ blocks
    let medium_bytes = MEDIUM_HZ * SIM_SECONDS * 32; //  ~960 KB → 1 DZ block
    let slow_bytes   = SLOW_HZ   * SIM_SECONDS * 32; //  ~192 KB → 1 DZ block

    println!("Simulation plan:");
    println!("  Fast   ({:>5} Hz × {} s): {:>6} records  {:>5.1} MB",
        FAST_HZ, SIM_SECONDS, FAST_HZ * SIM_SECONDS, fast_bytes as f64 / 1e6);
    println!("  Medium ({:>5} Hz × {} s): {:>6} records  {:>5.1} MB",
        MEDIUM_HZ, SIM_SECONDS, MEDIUM_HZ * SIM_SECONDS, medium_bytes as f64 / 1e6);
    println!("  Slow   ({:>5} Hz × {} s): {:>6} records  {:>5.1} MB",
        SLOW_HZ, SIM_SECONDS, SLOW_HZ * SIM_SECONDS, slow_bytes as f64 / 1e6);
    println!("  Compression: Deflate (threshold = 0, always on)");
    println!("  DZ block limit: 4 MB (MDF4 protocol requirement)");
    println!();

    // ─── Writer setup ─────────────────────────────────────────────────────────
    let metadata = Mf4Metadata::new()
        .with_author("MF4 Parse Demo")
        .with_organization("Test Organization")
        .with_project("Streaming Compression Demo")
        .with_comment("Compressed streaming write — multiple DZ blocks per DG");

    // block_size only controls DT (uncompressed) flushing; DZ blocks are
    // always capped at 4 MB internally per the MDF4 protocol.
    let config = StreamingConfig::new()
        .with_compression()
        .with_compression_threshold(0); // compress every block, even small ones

    let output_path = PathBuf::from("test/streaming_demo.mf4");
    let mut writer = Mf4StreamWriter::with_config(output_path.clone(), metadata, config)?;

    // ─── Channel group 1: Fast (5 kHz) — powertrain ───────────────────────────
    let cg_fast = ChannelGroupDefBuilder::new()
        .name("Powertrain_5kHz")
        .acq_source(
            SourceInfoBuilder::new()
                .name("ECU_Powertrain")
                .path("CAN1/Powertrain")
                .source_type(SourceType::Ecu)
                .comment("Engine management unit, 5 kHz crank-angle-synchronous sampling")
                .build()?,
        )
        .master(ChannelDef::new_master("time_fast"))
        .channel(ChannelDef::new("EngineSpeed").data_type(4).unit("rpm"))
        .channel(ChannelDef::new("Throttle").data_type(4).unit("%"))
        .channel(ChannelDef::new("MAP").data_type(4).unit("kPa"))
        .build()?;

    // ─── Channel group 2: Medium (500 Hz) — chassis ───────────────────────────
    let cg_medium = ChannelGroupDefBuilder::new()
        .name("Chassis_500Hz")
        .acq_source(
            SourceInfoBuilder::new()
                .name("ABS_Controller")
                .path("CAN2/Chassis")
                .source_type(SourceType::Bus)
                .bus_type(BusType::Can)
                .comment("ABS/ESC controller, 500 Hz wheel-speed synchronous sampling")
                .build()?,
        )
        .master(ChannelDef::new_master("time_medium"))
        .channel(ChannelDef::new("VehicleSpeed").data_type(4).unit("km/h"))
        .channel(ChannelDef::new("BrakePressure").data_type(4).unit("bar"))
        .channel(ChannelDef::new("SteeringAngle").data_type(4).unit("deg"))
        .build()?;

    // ─── Channel group 3: Slow (100 Hz) — thermal / fuel ─────────────────────
    let cg_slow = ChannelGroupDefBuilder::new()
        .name("Thermal_100Hz")
        .acq_source(
            SourceInfoBuilder::new()
                .name("ECU_Thermal")
                .path("ECU/Thermal")
                .source_type(SourceType::Ecu)
                .comment("Thermal and fuel management, 100 Hz polling rate")
                .build()?,
        )
        .master(ChannelDef::new_master("time_slow"))
        .channel(ChannelDef::new("EngineTemp").data_type(4).unit("°C"))
        .channel(ChannelDef::new("OilPressure").data_type(4).unit("bar"))
        .channel(ChannelDef::new("FuelLevel").data_type(4).unit("%"))
        .build()?;

    writer.add_data_group(StreamingDataGroup::new(cg_fast)?)?;
    writer.add_data_group(StreamingDataGroup::new(cg_medium)?)?;
    writer.add_data_group(StreamingDataGroup::new(cg_slow)?)?;
    writer.finalize_structure()?;

    println!("Structure written. Starting streaming data ({} simulated seconds)…\n",
        SIM_SECONDS);

    // ─── Streaming loop ───────────────────────────────────────────────────────
    // `write_duration` accumulates only the time spent inside MF4 write calls.
    // Any sleep (simulating real-time pacing) is excluded.
    let mut write_duration = Duration::ZERO;
    let mut noise: u64 = 0xDEAD_BEEF_1234_5678;

    for sec in 0..SIM_SECONDS {
        let t_write = Instant::now();

        // --- Fast: FAST_HZ records per simulated second -----------------------
        for i in 0..FAST_HZ {
            let t = (sec * FAST_HZ + i) as f64 / FAST_HZ as f64;
            writer.start_record(0, 0)?;
            writer.set_channel_value("time_fast", t)?;
            writer.set_channel_value("EngineSpeed", 800.0 + 5200.0 * (0.3 * t).sin().powi(2))?;
            writer.set_channel_value("Throttle",    20.0 + 80.0 * (0.5 * t).abs().sin())?;
            writer.set_channel_value("MAP",         90.0 + 30.0 * (0.7 * t).cos())?;
            writer.flush_record()?;
        }

        // --- Medium: MEDIUM_HZ records per simulated second -------------------
        for i in 0..MEDIUM_HZ {
            let t = (sec * MEDIUM_HZ + i) as f64 / MEDIUM_HZ as f64;
            writer.start_record(1, 0)?;
            writer.set_channel_value("time_medium", t)?;
            writer.set_channel_value("VehicleSpeed",  50.0 + 100.0 * (0.1 * t).sin().powi(2))?;
            writer.set_channel_value("BrakePressure",  5.0 +  25.0 * (0.4 * t).abs().sin())?;
            writer.set_channel_value("SteeringAngle", -90.0 * (0.2 * t).sin())?;
            writer.flush_record()?;
        }

        // --- Slow: SLOW_HZ records per simulated second -----------------------
        for i in 0..SLOW_HZ {
            let t = (sec * SLOW_HZ + i) as f64 / SLOW_HZ as f64;
            noise = noise.wrapping_mul(6364136223846793005).wrapping_add(1);
            let jitter = (noise >> 33) as f64 / u32::MAX as f64 * 0.5; // 0..0.5
            writer.start_record(2, 0)?;
            writer.set_channel_value("time_slow", t)?;
            writer.set_channel_value("EngineTemp",   85.0 + 15.0 * (0.05 * t).sin() + jitter)?;
            writer.set_channel_value("OilPressure",   3.5 +  1.5 * (0.08 * t).cos())?;
            writer.set_channel_value("FuelLevel",   100.0 -  t / SIM_SECONDS as f64 * 20.0)?;
            writer.flush_record()?;
        }

        write_duration += t_write.elapsed();

        // Simulate real-time pacing with a short sleep (not counted above).
        std::thread::sleep(Duration::from_millis(1));

        if (sec + 1) % 10 == 0 {
            println!("  {:>3} s written  (write time so far: {:.3} s)",
                sec + 1, write_duration.as_secs_f64());
        }
    }

    // ─── Finalize (write DL/HL/DZ chain, update DG links) ────────────────────
    let t_fin = Instant::now();
    writer.finalize_with_compact(false)?; // non-compact → DL-chained DZ blocks
    write_duration += t_fin.elapsed();

    // ─── Summary ──────────────────────────────────────────────────────────────
    let total_records = writer.total_records();
    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    println!("\n=== Write complete ===");
    println!("  MF4 write time : {:.3} s  (sleep time excluded)",
        write_duration.as_secs_f64());
    println!("  Total records  : {}", total_records);
    println!("  Throughput     : {:.0} records/s",
        total_records as f64 / write_duration.as_secs_f64());
    println!("  Output file    : {} ({:.2} MB)",
        output_path.display(), file_size as f64 / 1e6);
    println!("  Expected DZ blocks (fast group): ≥ {} (each ≤ 4 MB)",
        (fast_bytes + 4 * 1024 * 1024 - 1) / (4 * 1024 * 1024));

    Ok(())
}

