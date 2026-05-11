//! Example: Writing Value-to-Text (VTAB) and Value-Range-to-Text channels
//!
//! Demonstrates CC type 7 (exact key → text) and CC type 8 (range → text)
//! using three different APIs:
//!
//!   1. `SimpleWriter`  — ergonomic single-DG/single-CG façade
//!   2. `Mf4StreamWriter` — low-level streaming writer with explicit CG layout
//!   3. `Mf4Builder`    — one-time block writer (all data provided upfront)
//!
//! Raw numeric values are stored in the DT block; the CC block maps each raw
//! value (or range) to its display string for tools like INCA / CANalyzer.
//!
//! Run with:
//!   cargo run --release --example vtab_vrange_demo --features "streaming,compression"
//!
//! Output files are written to test/ and removed at the end.

#![cfg(feature = "streaming")]

use std::path::PathBuf;

use mf4_parse::writer::{
    Mf4Metadata, Mf4StreamWriter, StreamingConfig,
};
use mf4_parse::writer::stream_writer::{ChannelGroupDefBuilder, StreamingDataGroup};
use mf4_parse::writer::simple_writer::SimpleWriter;

// ─── helpers ────────────────────────────────────────────────────────────────

fn remove(path: &PathBuf) {
    if path.exists() { let _ = std::fs::remove_file(path); }
}

fn meta() -> Mf4Metadata {
    Mf4Metadata::new()
}

// ─── 1. SimpleWriter ────────────────────────────────────────────────────────

fn demo_simple_writer() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 1. SimpleWriter ---");

    let path = PathBuf::from("test/vtab_vrange_simple.mf4");
    remove(&path);

    let mut writer = SimpleWriter::new(&path)
        .time_channel("time", "s")
        // CC type 7 — exact raw value → label
        .vtab_u8_channel(
            "gear",
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            vec!["1st".into(), "2nd".into(), "3rd".into(), "4th".into(), "5th".into()],
            "N/A",
        )
        // CC type 8 — temperature range → text band (f64, 64-bit)
        .vrange_channel(
            "temp_band",
            4, 64,
            vec![(f64::NEG_INFINITY, 20.0), (20.0, 60.0), (60.0, f64::INFINITY)],
            vec!["Cold".into(), "Normal".into(), "Hot".into()],
            "?",
        )
        .build()?;

    let samples: &[(f64, f64, f64)] = &[
        (0.0,  1.0, 15.0),
        (0.1,  2.0, 35.0),
        (0.2,  3.0, 72.0),
        (0.3,  4.0, 55.0),
        (0.4,  5.0, 18.0),
    ];
    for &(t, g, temp) in samples {
        writer.write_record(&[t, g, temp])?;
    }
    writer.finalize()?;

    println!("  Written {} records → {:?}", samples.len(), path);
    // remove(&path);
    Ok(())
}

// ─── 2. Mf4StreamWriter ─────────────────────────────────────────────────────

fn demo_stream_writer() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 2. Mf4StreamWriter ---");

    let path = PathBuf::from("test/vtab_vrange_stream.mf4");
    remove(&path);

    // Build channel group: time + gear (vtab) + temp_band (vrange)
    let cg = ChannelGroupDefBuilder::new()
        .name("Vehicle")
        .with_time_channel("time")
        // CC type 7: gear position labels
        .add_vtab_channel(
            "gear", 0, 8,
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            vec!["P".into(), "1st".into(), "2nd".into(), "3rd".into(), "4th".into(), "5th".into()],
            "N/A",
        )
        // CC type 8: temperature classification
        .add_vrange_channel(
            "temp_band", 4, 64,
            vec![(f64::NEG_INFINITY, 20.0), (20.0, 60.0), (60.0, f64::INFINITY)],
            vec!["Cold".into(), "Normal".into(), "Hot".into()],
            "?",
        )
        .build()?;

    let config = StreamingConfig::new();
    let mut writer = Mf4StreamWriter::with_config(path.clone(), meta(), config)?;
    writer.add_data_group(StreamingDataGroup::new(cg)?)?;
    writer.finalize_structure()?;

    let samples: &[(f64, f64, f64)] = &[
        (0.00, 0.0,  5.0),  // gear=P, Cold
        (0.05, 1.0, 25.0),  // gear=1st, Normal
        (0.10, 2.0, 65.0),  // gear=2nd, Hot
        (0.15, 3.0, 50.0),  // gear=3rd, Normal
        (0.20, 4.0, 80.0),  // gear=4th, Hot
        (0.25, 5.0, 10.0),  // gear=5th, Cold
    ];

    for &(t, gear, temp) in samples {
        writer.start_record(0, 0)?;
        writer.set_channel_value("time",      t)?;
        writer.set_channel_value("gear",      gear)?;
        writer.set_channel_value("temp_band", temp)?;
        writer.flush_record()?;
    }
    writer.finalize()?;

    println!("  Written {} records → {:?}", samples.len(), path);
    remove(&path);
    Ok(())
}

// ─── main ────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== VTAB / VRange-to-Text Demo ===\n");

    demo_simple_writer()?;
    demo_stream_writer()?;

    println!("\nDone.");
    Ok(())
}
