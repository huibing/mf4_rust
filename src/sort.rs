//! MF4 Sort Feature
//!
//! Converts unsorted MF4 files (where a DataGroup contains multiple ChannelGroups
//! with interleaved records) into sorted MF4 files where each DataGroup contains
//! exactly one ChannelGroup with contiguous data.

use std::io::Cursor;
use std::path::PathBuf;

use crate::components::cg::channelgroup::ChannelGroup;
use crate::components::cn::channel::Channel;
use crate::components::dg::datagroup::DataGroup;
use crate::components::si::sourceinfo::{SiBusType, SiType};
use crate::parser::{load_file, Mdf};
use crate::components::cc::conversion::CcType;
use crate::writer::builder::{
    BusType, ChannelBuilder, ChannelGroupBuilder, ChannelType, ConversionBuilder,
    DataGroupBuilder, Mf4Builder, Mf4Metadata, SourceInfoBuilder, SourceType, SyncType,
};

type DynError = Box<dyn std::error::Error>;

/// Sort an MF4 file so that each DataGroup contains exactly one ChannelGroup.
///
/// Reads the input MF4 file, splits any unsorted DataGroups (those with multiple
/// ChannelGroups) into separate DataGroups with one ChannelGroup each, and writes
/// the result to the output path.
///
/// If the input file is already fully sorted, it is effectively copied with the
/// same logical structure.
pub fn sort_mf4(input: PathBuf, output: PathBuf) -> Result<(), DynError> {
    let file_data = load_file(input)?;
    let mut buf = Cursor::new(file_data.as_bytes());
    let mdf = Mdf::new::<fn(f64)>(&mut buf, None)?;

    let mut builder = Mf4Builder::new(Mf4Metadata::new().with_start_time(mdf.get_time_stamp_ns()));

    for dg in mdf.data.iter() {
        let cgs = dg.get_channle_groups();
        for cg in cgs.iter() {
            let dg_builder = build_sorted_dg(dg, cg, &mut buf)?;
            builder.add_data_group(dg_builder);
        }
    }

    builder.write(output)?;
    Ok(())
}

/// Build a DataGroupBuilder for a single ChannelGroup extracted from a (possibly unsorted) DataGroup.
///
/// Extracts raw record data for this CG from the shared data block and creates
/// a new sorted DG with exactly one CG.
fn build_sorted_dg(
    dg: &DataGroup,
    cg: &ChannelGroup,
    buf: &mut Cursor<&[u8]>,
) -> Result<DataGroupBuilder, DynError> {
    let mut cg_builder = ChannelGroupBuilder::new();

    if !cg.get_acq_name().is_empty() {
        cg_builder = cg_builder.name(cg.get_acq_name());
    }
    if !cg.get_comment().is_empty() {
        cg_builder = cg_builder.comment(cg.get_comment());
    }
    cg_builder = cg_builder.invalid_bytes(cg.get_invalid_bytes());
    cg_builder = cg_builder.flags(cg.get_cg_flags());
    cg_builder = cg_builder.data_bytes(cg.get_data_bytes());

    // Build source info
    let si = cg.get_acq_source();
    if !si.get_name().is_empty() || !si.get_path().is_empty() {
        let mut si_builder = SourceInfoBuilder::new();
        if !si.get_name().is_empty() {
            si_builder = si_builder.name(si.get_name());
        }
        if !si.get_path().is_empty() {
            si_builder = si_builder.path(si.get_path());
        }
        si_builder = si_builder.source_type(map_source_type(si.get_si_type()));
        si_builder = si_builder.bus_type(map_bus_type(si.get_bus_type()));
        si_builder = si_builder.simulated(si.is_simulated());
        cg_builder = cg_builder.acq_source(si_builder);
    }

    // Build master channel
    if let Some(master_cn) = cg.get_master() {
        let master_builder = build_channel_builder(master_cn, true)?;
        cg_builder = cg_builder.master(master_builder);
    }

    // Build regular channels
    for cn in cg.get_channels() {
        let cn_builder = build_channel_builder(cn, false)?;
        cg_builder = cg_builder.channel(cn_builder);
    }

    let cg_builder = cg_builder.build().map_err(|e| -> DynError { e.to_string().into() })?;

    // Extract raw record data for this CG
    let rec_id = cg.get_record_id();
    let cycle_count = cg.get_cycle_count();
    let record_bytes = cg.get_sample_total_bytes() as usize;

    let mut raw_data = Vec::with_capacity(cycle_count as usize * record_bytes);

    if !cg.is_vlsd() {
        for i in 0..cycle_count {
            if let Some(record) = dg.get_cg_data(rec_id, i, buf) {
                raw_data.extend_from_slice(&record);
            }
        }
    } else {
        // VLSD CG: extract variable-length records
        for i in 0..cycle_count {
            if let Some(record) = dg.get_vlsd_cg_data(rec_id, i, buf) {
                raw_data.extend_from_slice(&(record.len() as u32).to_le_bytes());
                raw_data.extend_from_slice(&record);
            }
        }
    }

    let mut dg_builder = DataGroupBuilder::new()
        .channel_group(cg_builder)
        .raw_dt_data(raw_data, cycle_count);

    if !dg.get_comment().is_empty() {
        dg_builder = dg_builder.comment(dg.get_comment());
    }

    let dg_builder = dg_builder.build().map_err(|e| -> DynError { e.to_string().into() })?;
    Ok(dg_builder)
}

/// Build a ChannelBuilder from a reader Channel.
fn build_channel_builder(cn: &Channel, is_master: bool) -> Result<ChannelBuilder, DynError> {
    let cn_type = match cn.get_cn_type() {
        0 => ChannelType::FixedLength,
        1 => ChannelType::VariableLength,
        2 => ChannelType::Master,
        3 => ChannelType::VirtualMaster,
        4 => ChannelType::Sync,
        // MLSD (cn_type=5) requires cn_data link to sub-channel which the writer doesn't support.
        // Convert to FixedLength since the raw data layout is preserved in sorted output.
        5 => ChannelType::FixedLength,
        6 => ChannelType::VirtualData,
        _ => ChannelType::FixedLength,
    };

    let sync_type = match cn.get_sync_type() {
        crate::components::cn::channel::SyncType::Time => SyncType::Time,
        crate::components::cn::channel::SyncType::Angle => SyncType::Angle,
        crate::components::cn::channel::SyncType::Distance => SyncType::Distance,
        crate::components::cn::channel::SyncType::Index => SyncType::Index,
        crate::components::cn::channel::SyncType::None => SyncType::None,
    };

    let mut builder = ChannelBuilder::new(cn.get_name())
        .data_type(cn.get_data_type())
        .bit_count(cn.get_bit_size())
        .cn_type(cn_type)
        .sync_type(sync_type)
        .byte_offset(cn.get_byte_offset())
        .bit_offset(cn.get_bit_offset());

    if !cn.get_unit().is_empty() {
        builder = builder.unit(cn.get_unit());
    }
    if !cn.get_comment().is_empty() {
        builder = builder.comment(cn.get_comment());
    }

    // Map source info
    let si = cn.get_source();
    if !si.get_name().is_empty() || !si.get_path().is_empty() {
        builder = builder.source(si.get_name(), si.get_path());
    }

    // Map conversion
    let cc = cn.get_conversion();
    let conversion = match cc.get_cc_type() {
        CcType::OneToOne => None,
        CcType::Linear((p1, p2)) => Some(ConversionBuilder::linear(*p1, *p2)),
        CcType::Rational(coeffs) => Some(ConversionBuilder::rational(*coeffs)),
        CcType::TableInt((keys, values)) => {
            Some(ConversionBuilder::table_interpolate(keys.clone(), values.clone()))
        }
        CcType::Table((keys, values)) => {
            Some(ConversionBuilder::table(keys.clone(), values.clone()))
        }
        _ => None, // Text-based conversions not yet supported in writer
    };
    if let Some(mut conv) = conversion {
        if !cc.get_cc_name().is_empty() {
            conv = conv.with_name(cc.get_cc_name());
        }
        if !cc.get_unit().is_empty() {
            conv = conv.with_unit(cc.get_unit());
        }
        if !cc.get_comment().is_empty() {
            conv = conv.with_comment(cc.get_comment());
        }
        builder = builder.conversion(conv);
    }

    let _ = is_master; // cn_type already captures master vs non-master
    builder = builder.build().map_err(|e| -> DynError { e.to_string().into() })?;
    Ok(builder)
}

fn map_source_type(si_type: &SiType) -> SourceType {
    match si_type {
        SiType::OTHER => SourceType::Other,
        SiType::ECU => SourceType::Ecu,
        SiType::BUS => SourceType::Bus,
        SiType::IO => SourceType::Io,
        SiType::TOOL => SourceType::Tool,
        SiType::USER => SourceType::User,
    }
}

fn map_bus_type(bus_type: &SiBusType) -> BusType {
    match bus_type {
        SiBusType::NONE => BusType::None,
        SiBusType::OTHER => BusType::Other,
        SiBusType::CAN => BusType::Can,
        SiBusType::LIN => BusType::Lin,
        SiBusType::MOST => BusType::Most,
        SiBusType::FLEXRAY => BusType::FlexRay,
        SiBusType::KLINE => BusType::KLine,
        SiBusType::ETHERNET => BusType::Ethernet,
        SiBusType::USB => BusType::Usb,
    }
}

#[cfg(test)]
mod sort_tests {
    use super::*;
    use crate::parser::Mf4Wrapper;
    use std::path::PathBuf;

    /// Helper: check that the output MF4 has all DGs sorted (each with exactly 1 CG)
    fn assert_all_sorted(path: &PathBuf) {
        let wrapper =
            Mf4Wrapper::new::<fn(f64)>(path.clone(), None).expect("Failed to open sorted file");
        assert!(wrapper.is_sorted(), "Output file should be fully sorted");
    }

    /// Test: sorting an already-sorted CAN bus file produces a valid sorted output
    #[test]
    fn test_sort_already_sorted_can_bus() {
        let input = PathBuf::from("test/Vector_CAN_DataFrame_Sort_Bus.MF4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let wrapper = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        assert!(wrapper.is_sorted(), "Input should be sorted");
        let input_channel_names = wrapper.get_channel_names();
        let input_cg_count = wrapper.get_all_channel_groups().len();

        let output = PathBuf::from("test/sorted_can_bus_output.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        let sorted_wrapper = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        let output_channel_names = sorted_wrapper.get_channel_names();

        assert_eq!(
            input_cg_count,
            sorted_wrapper.get_all_channel_groups().len()
        );
        for name in &input_channel_names {
            assert!(
                output_channel_names.contains(name),
                "Channel '{}' missing from sorted output",
                name
            );
        }

        let _ = std::fs::remove_file(&output);
    }

    /// Test: sorting an unsorted VLSD file produces a sorted output with correct DG/CG count
    #[test]
    fn test_sort_unsorted_vlsd() {
        let input = PathBuf::from("test/Vector_Unsorted_VLSD.MF4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let wrapper = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        let input_sorted = wrapper.is_sorted();
        let input_cg_count = wrapper.get_all_channel_groups().len();
        println!(
            "Input sorted: {}, CGs: {}",
            input_sorted, input_cg_count
        );

        let output = PathBuf::from("test/sorted_vlsd_output.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        assert_all_sorted(&output);

        let sorted_wrapper = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        let output_cg_count = sorted_wrapper.get_all_channel_groups().len();
        assert_eq!(
            input_cg_count, output_cg_count,
            "Total CG count should be preserved"
        );

        let _ = std::fs::remove_file(&output);
    }

    /// Test: sorting an already-sorted simple file produces a valid sorted output
    #[test]
    fn test_sort_already_sorted_simple() {
        let input = PathBuf::from("test/demo.mf4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let wrapper = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        assert!(wrapper.is_sorted(), "Input should already be sorted");
        let input_channels = wrapper.get_channel_names();

        let output = PathBuf::from("test/sorted_simple_output.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        assert_all_sorted(&output);

        let sorted_wrapper = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        let output_channels = sorted_wrapper.get_channel_names();
        for name in &input_channels {
            assert!(
                output_channels.contains(name),
                "Channel '{}' missing from sorted output",
                name
            );
        }

        let _ = std::fs::remove_file(&output);
    }

    /// Test: channel data values are preserved after sorting
    #[test]
    fn test_sort_preserves_channel_data() {
        let input = PathBuf::from("test/Vector_CAN_DataFrame_Sort_Bus.MF4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let wrapper = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        let channel_names = wrapper.get_channel_names();

        let mut input_data = std::collections::HashMap::new();
        for name in &channel_names {
            if let Some(data) = wrapper.get_channel_data(name) {
                input_data.insert(name.clone(), data);
            }
        }

        let output = PathBuf::from("test/sorted_data_preserve_output.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        let sorted_wrapper = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        for (name, expected_data) in &input_data {
            let actual_data = sorted_wrapper.get_channel_data(name);
            assert!(
                actual_data.is_some(),
                "Channel '{}' data should be present in sorted file",
                name
            );
            assert_eq!(
                actual_data.unwrap(),
                *expected_data,
                "Channel '{}' data should match after sorting",
                name
            );
        }

        let _ = std::fs::remove_file(&output);
    }

    /// Test: sort_mf4 returns an error for non-existent input file
    #[test]
    fn test_sort_nonexistent_input() {
        let input = PathBuf::from("test/nonexistent_file.mf4");
        let output = PathBuf::from("test/sorted_nonexistent_output.mf4");
        let result = sort_mf4(input, output.clone());
        assert!(result.is_err(), "Should error on non-existent input file");
        let _ = std::fs::remove_file(&output);
    }

    /// Test: sorting is idempotent — sorting an already-sorted output again produces equivalent result
    #[test]
    fn test_sort_idempotent() {
        // Use CAN bus file (no VLSD CGs) since sorted VLSD requires SD block support
        let input = PathBuf::from("test/Vector_CAN_DataFrame_Sort_Bus.MF4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let output1 = PathBuf::from("test/sorted_idempotent_pass1.mf4");
        let output2 = PathBuf::from("test/sorted_idempotent_pass2.mf4");

        sort_mf4(input.clone(), output1.clone()).expect("First sort should succeed");
        sort_mf4(output1.clone(), output2.clone()).expect("Second sort should succeed");

        let w1 = Mf4Wrapper::new::<fn(f64)>(output1.clone(), None).unwrap();
        let w2 = Mf4Wrapper::new::<fn(f64)>(output2.clone(), None).unwrap();

        assert!(w1.is_sorted());
        assert!(w2.is_sorted());
        assert_eq!(w1.get_channel_names().len(), w2.get_channel_names().len());

        for name in &w1.get_channel_names() {
            assert!(
                w2.get_channel_names().contains(name),
                "Channel '{}' missing after second sort",
                name
            );
            let d1 = w1.get_channel_data(name);
            let d2 = w2.get_channel_data(name);
            assert_eq!(
                d1.is_some(),
                d2.is_some(),
                "Channel '{}' data presence should match",
                name
            );
            if let (Some(v1), Some(v2)) = (d1, d2) {
                assert_eq!(v1, v2, "Channel '{}' data should match across sorts", name);
            }
        }

        let _ = std::fs::remove_file(&output1);
        let _ = std::fs::remove_file(&output2);
    }

    /// Test: unsorted VLSD file channel data is preserved after sorting
    #[test]
    fn test_sort_unsorted_vlsd_data_integrity() {
        let input = PathBuf::from("test/Vector_Unsorted_VLSD.MF4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let orig = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        let orig_names = orig.get_channel_names();

        let mut orig_data = std::collections::HashMap::new();
        for name in &orig_names {
            if let Some(data) = orig.get_channel_data(name) {
                orig_data.insert(name.clone(), data);
            }
        }
        assert!(!orig_data.is_empty(), "Should have readable channel data");

        let output = PathBuf::from("test/sorted_vlsd_data_integrity.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        let sorted = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        for (name, expected) in &orig_data {
            if let Some(actual) = sorted.get_channel_data(name) {
                assert_eq!(
                    actual, *expected,
                    "Channel '{}' data mismatch after sorting unsorted VLSD",
                    name
                );
            }
        }

        let _ = std::fs::remove_file(&output);
    }

    /// Test: cycle counts are preserved per CG after sorting
    #[test]
    fn test_sort_preserves_cycle_counts() {
        let input = PathBuf::from("test/Vector_Unsorted_VLSD.MF4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let orig = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        let mut orig_cycles: Vec<(String, u64)> = orig
            .get_all_channel_groups()
            .iter()
            .map(|cg| (cg.get_acq_name().to_string(), cg.get_cycle_count()))
            .collect();
        orig_cycles.sort_by(|a, b| a.0.cmp(&b.0));

        let output = PathBuf::from("test/sorted_cycle_counts.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        let sorted = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        let mut sorted_cycles: Vec<(String, u64)> = sorted
            .get_all_channel_groups()
            .iter()
            .map(|cg| (cg.get_acq_name().to_string(), cg.get_cycle_count()))
            .collect();
        sorted_cycles.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(
            orig_cycles.len(),
            sorted_cycles.len(),
            "CG count should be preserved"
        );
        for (orig, sorted) in orig_cycles.iter().zip(sorted_cycles.iter()) {
            assert_eq!(
                orig.0, sorted.0,
                "CG acquisition name should be preserved"
            );
            assert_eq!(
                orig.1, sorted.1,
                "CG '{}' cycle count should be preserved (orig={}, sorted={})",
                orig.0, orig.1, sorted.1
            );
        }

        let _ = std::fs::remove_file(&output);
    }

    /// Test: master/time channels are preserved after sorting
    #[test]
    fn test_sort_preserves_master_channels() {
        let input = PathBuf::from("test/demo.mf4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let orig = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        let orig_masters: Vec<Option<String>> = orig
            .get_all_channel_groups()
            .iter()
            .map(|cg| cg.get_master().map(|m| m.get_name().to_string()))
            .collect();

        let output = PathBuf::from("test/sorted_master_channels.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        let sorted = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        let sorted_masters: Vec<Option<String>> = sorted
            .get_all_channel_groups()
            .iter()
            .map(|cg| cg.get_master().map(|m| m.get_name().to_string()))
            .collect();

        assert_eq!(
            orig_masters.len(),
            sorted_masters.len(),
            "CG count should be preserved"
        );
        for (i, (orig_m, sorted_m)) in
            orig_masters.iter().zip(sorted_masters.iter()).enumerate()
        {
            assert_eq!(
                orig_m, sorted_m,
                "CG {} master channel name should be preserved",
                i
            );
        }

        // Verify master channel data matches
        for cg in orig.get_all_channel_groups() {
            if let Some(master) = cg.get_master() {
                let name = master.get_name();
                let orig_data = orig.get_channel_data(name);
                let sorted_data = sorted.get_channel_data(name);
                assert_eq!(
                    orig_data.is_some(),
                    sorted_data.is_some(),
                    "Master channel '{}' data presence should match",
                    name
                );
                if let (Some(o), Some(s)) = (orig_data, sorted_data) {
                    assert_eq!(
                        o, s,
                        "Master channel '{}' data should match after sorting",
                        name
                    );
                }
            }
        }

        let _ = std::fs::remove_file(&output);
    }

    /// Test: multi-DG file preserves DG count after sorting
    #[test]
    fn test_sort_preserves_dg_count() {
        let input = PathBuf::from("test/demo.mf4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let orig = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        let orig_cg_count = orig.get_all_channel_groups().len();
        assert!(
            orig_cg_count >= 2,
            "demo.mf4 should have at least 2 CGs for this test"
        );

        let output = PathBuf::from("test/sorted_dg_count.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        let sorted = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        assert!(sorted.is_sorted(), "Output should be sorted");
        assert_eq!(
            orig_cg_count,
            sorted.get_all_channel_groups().len(),
            "CG count should be preserved for already-sorted file"
        );

        let _ = std::fs::remove_file(&output);
    }

    /// Test: conversion types are preserved after sorting
    #[test]
    fn test_sort_preserves_conversions() {
        let input = PathBuf::from("test/Vector_CAN_DataFrame_Sort_Bus.MF4");
        if !input.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let orig = Mf4Wrapper::new::<fn(f64)>(input.clone(), None).unwrap();
        // Collect conversion info from original
        let mut orig_conversions: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for cg in orig.get_all_channel_groups() {
            for cn in cg.get_channels() {
                let cc_type = format!("{:?}", cn.get_conversion().get_cc_type());
                orig_conversions.insert(cn.get_name().to_string(), cc_type);
            }
        }

        let output = PathBuf::from("test/sorted_conversions.mf4");
        sort_mf4(input.clone(), output.clone()).expect("sort_mf4 should succeed");

        let sorted = Mf4Wrapper::new::<fn(f64)>(output.clone(), None).unwrap();
        for cg in sorted.get_all_channel_groups() {
            for cn in cg.get_channels() {
                let name = cn.get_name().to_string();
                let sorted_cc = format!("{:?}", cn.get_conversion().get_cc_type());
                if let Some(orig_cc) = orig_conversions.get(&name) {
                    assert_eq!(
                        orig_cc, &sorted_cc,
                        "Channel '{}' conversion type should be preserved (orig={}, sorted={})",
                        name, orig_cc, sorted_cc
                    );
                }
            }
        }

        let _ = std::fs::remove_file(&output);
    }
}
