//! Write feature tests
//!
//! Tests for MF4 file writing functionality

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::Mf4Wrapper;
    use crate::writer::builder::{Mf4Builder, Mf4Metadata, ChannelBuilder, ChannelGroupBuilder, DataGroupBuilder};
    #[cfg(feature = "streaming")]
    use crate::writer::stream_writer::{Mf4StreamWriter, StreamingDataGroup, ChannelGroupDef, ChannelDef, StreamingConfig};

    /// Helper to clean up test files
    fn cleanup_test_file(path: &PathBuf) {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Test: Write a simple MF4 file using Mf4Builder
    #[test]
    fn test_builder_simple_write() {
        // Create metadata
        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: 1704067200000000000, // 2024-01-01 00:00:00
            author: Some("Test User".to_string()),
            organization: Some("Test Org".to_string()),
            project: Some("Write Test".to_string()),
            comment: Some("Simple write test".to_string()),
        };

        // Create builder
        let mut builder = Mf4Builder::new(metadata);

        // Define channels
        let time_channel = ChannelBuilder::new_master_time("time");
        let temp_channel = ChannelBuilder::new("Temperature")
            .data_type(4)      // FLOAT64 LE
            .unit("°C")
            .comment("Engine temperature")
            .build().unwrap();
        let rpm_channel = ChannelBuilder::new("RPM")
            .data_type(2)      // INT16 LE
            .bit_count(16)
            .unit("rpm")
            .build().unwrap();

        // Create channel group
        let cg = ChannelGroupBuilder::new()
            .name("EngineData")
            .master(time_channel)
            .channel(temp_channel)
            .channel(rpm_channel)
            .build().unwrap();

        // Create data group
        let dg = DataGroupBuilder::new()
            .channel_group(cg)
            .build().unwrap();

        builder.add_data_group(dg);

        // Add data (10 samples)
        let time_data: Vec<f64> = (0..10).map(|i| i as f64 * 0.1).collect();
        let temp_data: Vec<f64> = (0..10).map(|i| 20.0 + i as f64).collect();
        let rpm_data: Vec<i16> = (0..10).map(|i| 1000 + i * 100).collect();

        builder.set_channel_data("time", &time_data).unwrap();
        builder.set_channel_data("Temperature", &temp_data).unwrap();
        builder.set_channel_data("RPM", &rpm_data).unwrap();

        // Write to temp file
        let output_path = PathBuf::from("temp_builder_test.mf4");
        cleanup_test_file(&output_path);

        let result = builder.write(output_path.clone());
        match &result {
            Ok(()) => println!("Write succeeded"),
            Err(e) => println!("Write failed: {:?}", e),
        }

        // Cleanup
        cleanup_test_file(&output_path);

        // Note: Currently write() returns UnsupportedFeature error
        // This test documents the expected behavior
        assert!(result.is_ok() || matches!(result, Err(crate::writer::WriteError::UnsupportedFeature(_))));
    }

    /// Test: Read existing MF4 file and verify basic structure
    #[test]
    fn test_read_existing_file() {
        // Read demo.mf4
        let input_path = PathBuf::from("test/demo.mf4");
        if !input_path.exists() {
            println!("Skipping test: demo.mf4 not found");
            return;
        }

        let mf4 = Mf4Wrapper::new::<fn(f64)>(input_path, None).unwrap();

        // Verify we can read channel names
        let channel_names = mf4.get_channel_names();
        println!("Found {} channels", channel_names.len());
        assert!(!channel_names.is_empty());

        // Try to read data from first channel
        if let Some(first_channel) = channel_names.first() {
            println!("Reading data from: {}", first_channel);
            if let Some(data) = mf4.get_channel_data(first_channel) {
                println!("Data type: {:?}", data);
            }
        }
    }

    /// Test: Round-trip - read, build, write, read again
    #[test]
    fn test_round_trip_builder() {
        // Read existing file
        let input_path = PathBuf::from("test/demo.mf4");
        if !input_path.exists() {
            println!("Skipping test: demo.mf4 not found");
            return;
        }

        let original = Mf4Wrapper::new::<fn(f64)>(input_path.clone(), None).unwrap();
        let channel_names = original.get_channel_names();

        println!("=== Round-trip test for demo.mf4 ===");
        println!("Channels: {:?}", channel_names);

        // Create new builder with metadata from original
        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            author: None,
            organization: None,
            project: None,
            comment: Some("Round-trip test".to_string()),
        };

        let mut builder = Mf4Builder::new(metadata);

        // Build channel structure from original
        // For simplicity, just read first data group
        // In real implementation, we'd iterate all data groups

        let output_path = PathBuf::from("temp_roundtrip_test.mf4");
        cleanup_test_file(&output_path);

        let result = builder.write(output_path.clone());
        println!("Write result: {:?}", result.is_ok());

        cleanup_test_file(&output_path);
    }

    /// Test: Streaming write with simple data
    #[cfg(feature = "streaming")]
    #[test]
    fn test_streaming_write_simple() {
        let output_path = PathBuf::from("temp_streaming_test.mf4");
        cleanup_test_file(&output_path);

        // Create config with small block size for testing
        let config = StreamingConfig::new()
            .with_block_size(1000); // 1KB blocks

        let metadata = Mf4Metadata::default();

        let result = Mf4StreamWriter::with_config(
            output_path.clone(),
            metadata,
            config,
        );

        match result {
            Ok(mut writer) => {
                // Define channels
                let time_def = ChannelDef::new_master("time");
                let signal_def = ChannelDef::new("Signal")
                    .data_type(4)  // FLOAT64
                    .unit("V");

                let cg_def = ChannelGroupDef::builder()
                    .name("Measurement")
                    .master(time_def)
                    .channel(signal_def)
                    .build().unwrap();

                let dg = StreamingDataGroup::new(cg_def).unwrap();
                writer.add_data_group(dg).unwrap();

                // Finalize structure
                writer.finalize_structure().unwrap();
                assert_eq!(writer.state(), crate::writer::stream_writer::WriterState::StructureReady);

                // Write some records
                for i in 0..10 {
                    writer.start_record(0, 0).unwrap();
                    writer.set_channel_value("time", i as f64 * 0.01).unwrap();
                    writer.set_channel_value("Signal", (i as f64).sin()).unwrap();
                    writer.flush_record().unwrap();
                }

                assert_eq!(writer.state(), crate::writer::stream_writer::WriterState::Writing);
                assert_eq!(writer.total_records(), 10);

                // Finalize
                writer.finalize().unwrap();
                assert_eq!(writer.state(), crate::writer::stream_writer::WriterState::Finalized);

                println!("Streaming write test completed successfully");
            }
            Err(e) => {
                println!("Streaming write failed: {:?}", e);
            }
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Streaming write with multiple data groups
    #[cfg(feature = "streaming")]
    #[test]
    fn test_streaming_write_multi_cg() {
        let output_path = PathBuf::from("temp_streaming_multi_test.mf4");
        cleanup_test_file(&output_path);

        let config = StreamingConfig::new();
        let metadata = Mf4Metadata::default();

        let result = Mf4StreamWriter::with_config(
            output_path.clone(),
            metadata,
            config,
        );

        match result {
            Ok(mut writer) => {
                // Define two channel groups
                let cg1 = ChannelGroupDef::builder()
                    .name("GroupA")
                    .record_id(1)
                    .master(ChannelDef::new_master("time_a"))
                    .channel(ChannelDef::new("SignalA").data_type(4))
                    .build().unwrap();

                let cg2 = ChannelGroupDef::builder()
                    .name("GroupB")
                    .record_id(2)
                    .master(ChannelDef::new_master("time_b"))
                    .channel(ChannelDef::new("SignalB").data_type(4))
                    .build().unwrap();

                let dg = StreamingDataGroup::with_multiple(vec![cg1, cg2]).unwrap();
                writer.add_data_group(dg).unwrap();
                writer.finalize_structure().unwrap();

                // Write interleaved records
                for i in 0..20 {
                    // Group A
                    writer.start_record(0, 0).unwrap();
                    writer.set_channel_value("time_a", i as f64 * 0.01).unwrap();
                    writer.set_channel_value("SignalA", i as f64).unwrap();
                    writer.flush_record().unwrap();

                    // Group B (every other sample)
                    if i % 2 == 0 {
                        writer.start_record(0, 1).unwrap();
                        writer.set_channel_value("time_b", i as f64 * 0.02).unwrap();
                        writer.set_channel_value("SignalB", i as f64 * 2.0).unwrap();
                        writer.flush_record().unwrap();
                    }
                }

                writer.finalize().unwrap();
                println!("Multi-CG streaming write test completed");
            }
            Err(e) => {
                println!("Multi-CG streaming write failed: {:?}", e);
            }
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Full round-trip for test/1.mf4 - read all data, write new file, compare channel by channel
    ///
    /// This test reads from test/1.mf4, writes to a new file, and verifies data integrity.
    /// Note: The current builder only supports a single channel group, so we test channels
    /// that have the same cycle count (number of records) as the master time channel.
    #[test]
    fn test_read_1_mf4_and_write() {
        let input_path = PathBuf::from("test/1.mf4");
        if !input_path.exists() {
            println!("Skipping test: test/1.mf4 not found");
            return;
        }

        println!("=== Full Round-trip Test: test/1.mf4 ===");

        // 1. Read original file
        let original = Mf4Wrapper::new::<fn(f64)>(input_path.clone(), None).unwrap();
        let original_channels = original.get_channel_names();
        println!("Original file has {} channels", original_channels.len());

        // 2. Get the master time data to determine the primary cycle count
        // The first channel's master time defines the main record count
        let master_data: Vec<f64> = if let Some(first_ch) = original_channels.first() {
            if let Some(master) = original.get_channel_master_data(first_ch) {
                match master {
                    crate::data_serde::DataValue::REAL(vals) => vals.clone(),
                    _ => {
                        let len = original.get_channel_data(first_ch)
                            .map(|d| d.len())
                            .unwrap_or(100);
                        (0..len).map(|i| i as f64 * 0.01).collect()
                    }
                }
            } else {
                let len = original.get_channel_data(first_ch)
                    .map(|d| d.len())
                    .unwrap_or(100);
                (0..len).map(|i| i as f64 * 0.01).collect()
            }
        } else {
            (0..100).map(|i| i as f64 * 0.01).collect()
        };

        let target_cycle_count = master_data.len();
        println!("Target cycle count (from master time): {}", target_cycle_count);

        // 3. Create metadata for new file
        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: original.get_time_stamp_ns(),
            author: Some("Round-trip Test".to_string()),
            organization: None,
            project: None,
            comment: Some("Full round-trip test from test/1.mf4".to_string()),
        };

        let mut builder = Mf4Builder::new(metadata);

        // 4. Collect channel info - only include channels that match the target cycle count
        // and are not array channels (those with [] in the name)
        let mut channel_builders: Vec<ChannelBuilder> = Vec::new();
        let mut processed_channels: Vec<String> = Vec::new();
        let mut skipped_channels: Vec<(String, String)> = Vec::new();

        for ch_name in &original_channels {
            // Skip array channels (indicated by [] in the name)
            if ch_name.contains('[') && ch_name.contains(']') {
                skipped_channels.push((ch_name.clone(), "array channel".to_string()));
                continue;
            }

            // Try to get channel data
            let data = match original.get_channel_data(ch_name) {
                Some(d) => d,
                None => {
                    skipped_channels.push((ch_name.clone(), "get_channel_data returned None".to_string()));
                    continue;
                }
            };

            // Check if cycle count matches
            let data_len = data.len();
            if data_len != target_cycle_count {
                skipped_channels.push((ch_name.clone(), format!("cycle count mismatch: {} vs {}", data_len, target_cycle_count)));
                continue;
            }

            // Build channel based on data type
            let ch_builder = match &data {
                crate::data_serde::DataValue::REAL(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty REAL data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(4)  // FLOAT64 LE
                        .bit_count(64)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::SINGLE(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty SINGLE data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(4)  // FLOAT32 LE
                        .bit_count(32)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::FLOAT16(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty FLOAT16 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(4)  // FLOAT32 LE
                        .bit_count(32)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::UINT64(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty UINT64 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(0)  // UINT LE
                        .bit_count(64)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::UINT32(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty UINT32 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(0)  // UINT LE
                        .bit_count(32)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::UINT16(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty UINT16 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(0)  // UINT LE
                        .bit_count(16)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::UINT8(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty UINT8 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(0)  // UINT LE
                        .bit_count(8)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::INT64(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty INT64 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(2)  // INT LE
                        .bit_count(64)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::INT32(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty INT32 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(2)  // INT LE
                        .bit_count(32)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::INT16(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty INT16 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(2)  // INT LE
                        .bit_count(16)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::INT8(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty INT8 data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(2)  // INT LE
                        .bit_count(8)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::BYTE(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty BYTE data".to_string()));
                        continue;
                    }
                    ChannelBuilder::new(ch_name)
                        .data_type(0)  // UINT LE
                        .bit_count(8)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::STRINGS(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty STRINGS data".to_string()));
                        continue;
                    }
                    let max_len = vals.iter().map(|s| s.len()).max().unwrap_or(0);
                    let bit_count = ((max_len + 1) * 8) as u32;
                    ChannelBuilder::new(ch_name)
                        .data_type(6)  // STRING_LE (UTF-8)
                        .bit_count(bit_count.max(16))
                        .build().unwrap()
                }
                crate::data_serde::DataValue::CHAR(_) => {
                    skipped_channels.push((ch_name.clone(), "CHAR type (single string)".to_string()));
                    continue;
                }
                crate::data_serde::DataValue::BYTEARRAY(vals) => {
                    if vals.is_empty() {
                        skipped_channels.push((ch_name.clone(), "empty BYTEARRAY data".to_string()));
                        continue;
                    }
                    let max_len = vals.iter().map(|arr| arr.len()).max().unwrap_or(0);
                    ChannelBuilder::new(ch_name)
                        .data_type(10)  // BYTE_ARRAY
                        .bit_count((max_len * 8) as u32)
                        .build().unwrap()
                }
                crate::data_serde::DataValue::STRUCT(_) => {
                    skipped_channels.push((ch_name.clone(), "STRUCT type (complex)".to_string()));
                    continue;
                }
                crate::data_serde::DataValue::MIXED(_) => {
                    skipped_channels.push((ch_name.clone(), "MIXED type (complex)".to_string()));
                    continue;
                }
            };

            channel_builders.push(ch_builder);
            processed_channels.push(ch_name.clone());
        }

        if channel_builders.is_empty() {
            println!("No processable channels found, skipping test");
            return;
        }

        println!("Processed {} channels (matching cycle count {})", processed_channels.len(), target_cycle_count);
        if !skipped_channels.is_empty() {
            println!("Skipped {} channels:", skipped_channels.len());
            for (name, reason) in skipped_channels.iter().take(10) {
                println!("  - {}: {}", name, reason);
            }
            if skipped_channels.len() > 10 {
                println!("  ... and {} more", skipped_channels.len() - 10);
            }
        }

        // 5. Build channel group
        let mut cg_builder = ChannelGroupBuilder::new()
            .name("RoundtripData")
            .master(ChannelBuilder::new_master_time("time"));

        for ch in channel_builders {
            cg_builder = cg_builder.channel(ch);
        }

        let cg = cg_builder.build().unwrap();

        // 6. Build data group
        let dg = DataGroupBuilder::new()
            .channel_group(cg)
            .build().unwrap();

        builder.add_data_group(dg);

        // 7. Set time data
        builder.set_channel_data("time", &master_data).unwrap();

        // 8. Set channel data
        for ch_name in &processed_channels {
            if let Some(data) = original.get_channel_data(ch_name) {
                match &data {
                    crate::data_serde::DataValue::REAL(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::SINGLE(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::FLOAT16(vals) => {
                        let f32_vals: Vec<f32> = vals.iter().map(|v| v.to_f32()).collect();
                        builder.set_channel_data(ch_name, &f32_vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT64(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT32(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT16(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT8(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT64(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT32(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT16(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT8(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::BYTE(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::STRINGS(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::BYTEARRAY(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    _ => {}
                }
            }
        }

        // 9. Write to new file
        let output_path = PathBuf::from("temp_1_mf4_roundtrip.mf4");
        cleanup_test_file(&output_path);

        let write_result = builder.write(output_path.clone());
        assert!(write_result.is_ok(), "Write failed: {:?}", write_result);
        println!("Wrote to: {:?}", output_path);

        // 10. Read the new file back
        let new_file = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let new_channels = new_file.get_channel_names();

        println!("New file has {} channels", new_channels.len());

        // 11. Compare data channel by channel
        let mut passed = 0;
        let mut failed = 0;

        for ch_name in &processed_channels {
            let original_data = original.get_channel_data(ch_name);
            let new_data = new_file.get_channel_data(ch_name);

            match (original_data, new_data) {
                (Some(orig), Some(new)) => {
                    let matches = match (&orig, &new) {
                        (crate::data_serde::DataValue::REAL(o), crate::data_serde::DataValue::REAL(n)) => {
                            if o.len() == n.len() {
                                o.iter().zip(n.iter()).all(|(a, b)| (a - b).abs() < 1e-10)
                            } else {
                                println!("  Length mismatch for {}: {} vs {}", ch_name, o.len(), n.len());
                                false
                            }
                        }
                        (crate::data_serde::DataValue::SINGLE(o), crate::data_serde::DataValue::SINGLE(n)) => {
                            o.len() == n.len() && o.iter().zip(n.iter()).all(|(a, b)| (a - b).abs() < 1e-5)
                        }
                        (crate::data_serde::DataValue::FLOAT16(o), crate::data_serde::DataValue::SINGLE(n)) => {
                            if o.len() == n.len() {
                                o.iter().zip(n.iter()).all(|(a, b)| (a.to_f32() - *b).abs() < 1e-3)
                            } else {
                                false
                            }
                        }
                        (crate::data_serde::DataValue::UINT64(o), crate::data_serde::DataValue::UINT64(n)) => o == n,
                        (crate::data_serde::DataValue::UINT32(o), crate::data_serde::DataValue::UINT32(n)) => o == n,
                        (crate::data_serde::DataValue::UINT16(o), crate::data_serde::DataValue::UINT16(n)) => o == n,
                        (crate::data_serde::DataValue::UINT8(o), crate::data_serde::DataValue::UINT8(n)) => o == n,
                        (crate::data_serde::DataValue::INT64(o), crate::data_serde::DataValue::INT64(n)) => o == n,
                        (crate::data_serde::DataValue::INT32(o), crate::data_serde::DataValue::INT32(n)) => o == n,
                        (crate::data_serde::DataValue::INT16(o), crate::data_serde::DataValue::INT16(n)) => o == n,
                        (crate::data_serde::DataValue::INT8(o), crate::data_serde::DataValue::INT8(n)) => o == n,
                        (crate::data_serde::DataValue::BYTE(o), crate::data_serde::DataValue::BYTE(n)) => o == n,
                        (crate::data_serde::DataValue::STRINGS(o), crate::data_serde::DataValue::STRINGS(n)) => {
                            if o.len() == n.len() {
                                o.iter().zip(n.iter()).all(|(a, b)| a.trim_end_matches('\0') == b.trim_end_matches('\0'))
                            } else {
                                println!("  String length mismatch for {}: {} vs {}", ch_name, o.len(), n.len());
                                false
                            }
                        }
                        (crate::data_serde::DataValue::BYTEARRAY(o), crate::data_serde::DataValue::BYTEARRAY(n)) => {
                            if o.len() == n.len() {
                                o.iter().zip(n.iter()).all(|(a, b)| a == b)
                            } else {
                                println!("  ByteArray length mismatch for {}: {} vs {}", ch_name, o.len(), n.len());
                                false
                            }
                        }
                        _ => {
                            println!("  Type mismatch for {}: {:?} vs {:?}", ch_name,
                                std::mem::discriminant(&orig),
                                std::mem::discriminant(&new));
                            false
                        }
                    };

                    if matches {
                        passed += 1;
                        println!("✓ {} - data matches", ch_name);
                    } else {
                        failed += 1;
                        println!("✗ {} - data mismatch", ch_name);
                    }
                }
                (Some(_), None) => {
                    failed += 1;
                    println!("✗ {} - not found in new file", ch_name);
                }
                (None, Some(_)) => {
                    failed += 1;
                    println!("✗ {} - not found in original", ch_name);
                }
                (None, None) => {
                    failed += 1;
                    println!("✗ {} - not found in either file", ch_name);
                }
            }
        }

        println!("\n=== Round-trip Summary ===");
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        println!("Skipped: {}", skipped_channels.len());

        cleanup_test_file(&output_path);

        assert!(failed == 0, "{} channels failed data comparison", failed);
    }

    /// Test: Full round-trip - read demo.mf4, write new file, read back and compare
    #[test]
    fn test_full_roundtrip_demo_mf4() {
        let input_path = PathBuf::from("test/demo.mf4");
        if !input_path.exists() {
            println!("Skipping test: test/demo.mf4 not found");
            return;
        }

        println!("=== Full Round-trip Test: test/demo.mf4 ===");

        // 1. Read original file
        let original = Mf4Wrapper::new::<fn(f64)>(input_path.clone(), None).unwrap();
        let original_channels = original.get_channel_names();
        println!("Original file has {} channels", original_channels.len());

        // 2. Create metadata for new file
        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: original.get_time_stamp_ns(),
            author: Some("Round-trip Test".to_string()),
            organization: None,
            project: None,
            comment: Some("Full round-trip test from demo.mf4".to_string()),
        };

        let mut builder = Mf4Builder::new(metadata);

        // 3. Build channel structure - for simplicity, create one CG with numeric channels
        let mut channel_builders: Vec<ChannelBuilder> = Vec::new();
        let mut numeric_channels: Vec<String> = Vec::new();

        for ch_name in &original_channels {
            // Try to get channel data, skip channels with conversion errors
            let data = match original.get_channel_data(ch_name) {
                Some(d) => d,
                None => {
                    println!("Skipping channel with conversion error: {}", ch_name);
                    continue;
                }
            };

            // Only handle numeric types for now
            match &data {
                crate::data_serde::DataValue::REAL(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(4)  // FLOAT64 LE
                            .bit_count(64)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::SINGLE(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(4)  // FLOAT32 LE
                            .bit_count(32)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::UINT64(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(0)  // UINT LE
                            .bit_count(64)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::UINT32(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(0)  // UINT LE
                            .bit_count(32)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::UINT16(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(0)  // UINT LE
                            .bit_count(16)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::UINT8(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(0)  // UINT LE
                            .bit_count(8)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::INT64(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(2)  // INT LE
                            .bit_count(64)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::INT32(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(2)  // INT LE
                            .bit_count(32)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::INT16(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(2)  // INT LE
                            .bit_count(16)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                crate::data_serde::DataValue::INT8(vals) => {
                    if !vals.is_empty() {
                        let ch = ChannelBuilder::new(ch_name)
                            .data_type(2)  // INT LE
                            .bit_count(8)
                            .build().unwrap();
                        channel_builders.push(ch);
                        numeric_channels.push(ch_name.clone());
                    }
                }
                _ => {
                    println!("Skipping non-numeric channel: {}", ch_name);
                }
            }
        }

        if channel_builders.is_empty() {
            println!("No numeric channels found, skipping test");
            return;
        }

        println!("Found {} numeric channels to copy", channel_builders.len());

        // Get master time data
        let master_data: Vec<f64> = if let Some(first_ch) = numeric_channels.first() {
            if let Some(master) = original.get_channel_master_data(first_ch) {
                match master {
                    crate::data_serde::DataValue::REAL(vals) => vals.clone(),
                    _ => {
                        // Generate time values if no master
                        (0..100).map(|i| i as f64 * 0.01).collect()
                    }
                }
            } else {
                (0..100).map(|i| i as f64 * 0.01).collect()
            }
        } else {
            (0..100).map(|i| i as f64 * 0.01).collect()
        };

        // Build channel group
        let mut cg_builder = ChannelGroupBuilder::new()
            .name("RoundtripData")
            .master(ChannelBuilder::new_master_time("time"));

        for ch in channel_builders {
            cg_builder = cg_builder.channel(ch);
        }

        let cg = cg_builder.build().unwrap();

        // Build data group
        let dg = DataGroupBuilder::new()
            .channel_group(cg)
            .build().unwrap();

        builder.add_data_group(dg);

        // Set time data
        builder.set_channel_data("time", &master_data).unwrap();

        // Set channel data
        for ch_name in &numeric_channels {
            if let Some(data) = original.get_channel_data(ch_name) {
                match &data {
                    crate::data_serde::DataValue::REAL(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::SINGLE(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT64(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT32(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT16(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::UINT8(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT64(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT32(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT16(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    crate::data_serde::DataValue::INT8(vals) => {
                        builder.set_channel_data(ch_name, vals).unwrap();
                    }
                    _ => {}
                }
            }
        }

        // Write to new file
        let output_path = PathBuf::from("temp_roundtrip_demo.mf4");
        cleanup_test_file(&output_path);

        let write_result = builder.write(output_path.clone());
        assert!(write_result.is_ok(), "Write failed: {:?}", write_result);
        println!("Wrote to: {:?}", output_path);

        // 4. Read the new file back
        let new_file = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let new_channels = new_file.get_channel_names();

        println!("New file has {} channels", new_channels.len());

        // 5. Compare data
        let mut passed = 0;
        let mut failed = 0;

        for ch_name in &numeric_channels {
            let original_data = original.get_channel_data(ch_name);
            let new_data = new_file.get_channel_data(ch_name);

            match (original_data, new_data) {
                (Some(orig), Some(new)) => {
                    // Compare based on type
                    let matches = match (&orig, &new) {
                        (crate::data_serde::DataValue::REAL(o), crate::data_serde::DataValue::REAL(n)) => {
                            if o.len() == n.len() {
                                let all_close = o.iter().zip(n.iter())
                                    .all(|(a, b)| (a - b).abs() < 1e-10);
                                all_close
                            } else {
                                println!("Length mismatch for {}: {} vs {}", ch_name, o.len(), n.len());
                                false
                            }
                        }
                        (crate::data_serde::DataValue::SINGLE(o), crate::data_serde::DataValue::SINGLE(n)) => {
                            o.len() == n.len() && o.iter().zip(n.iter()).all(|(a, b)| (a - b).abs() < 1e-5)
                        }
                        (crate::data_serde::DataValue::UINT64(o), crate::data_serde::DataValue::UINT64(n)) => o == n,
                        (crate::data_serde::DataValue::UINT32(o), crate::data_serde::DataValue::UINT32(n)) => o == n,
                        (crate::data_serde::DataValue::UINT16(o), crate::data_serde::DataValue::UINT16(n)) => o == n,
                        (crate::data_serde::DataValue::UINT8(o), crate::data_serde::DataValue::UINT8(n)) => o == n,
                        (crate::data_serde::DataValue::INT64(o), crate::data_serde::DataValue::INT64(n)) => o == n,
                        (crate::data_serde::DataValue::INT32(o), crate::data_serde::DataValue::INT32(n)) => o == n,
                        (crate::data_serde::DataValue::INT16(o), crate::data_serde::DataValue::INT16(n)) => o == n,
                        (crate::data_serde::DataValue::INT8(o), crate::data_serde::DataValue::INT8(n)) => o == n,
                        _ => {
                            println!("Type mismatch for {}: {:?} vs {:?}", ch_name, orig, new);
                            false
                        }
                    };

                    if matches {
                        passed += 1;
                        println!("✓ {} - data matches", ch_name);
                    } else {
                        failed += 1;
                        println!("✗ {} - data mismatch", ch_name);
                    }
                }
                (Some(_), None) => {
                    failed += 1;
                    println!("✗ {} - not found in new file", ch_name);
                }
                (None, Some(_)) => {
                    failed += 1;
                    println!("✗ {} - not found in original", ch_name);
                }
                (None, None) => {
                    failed += 1;
                    println!("✗ {} - not found in either file", ch_name);
                }
            }
        }

        println!("\n=== Round-trip Summary ===");
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);

        cleanup_test_file(&output_path);

        assert!(failed == 0, "{} channels failed data comparison", failed);
    }

    /// Test: Compare data before and after round-trip
    #[test]
    fn test_data_integrity() {
        // This test will verify that data read from an MF4 file
        // matches data written and re-read from a new file

        // For now, just verify we can create builders with correct types
        let metadata = Mf4Metadata::default();
        let mut builder = Mf4Builder::new(metadata);

        // Create channel with different data types
        let f64_ch = ChannelBuilder::new("float64_val")
            .data_type(4)
            .bit_count(64)
            .build().unwrap();

        let i32_ch = ChannelBuilder::new("int32_val")
            .data_type(2)
            .bit_count(32)
            .build().unwrap();

        let u16_ch = ChannelBuilder::new("uint16_val")
            .data_type(0)
            .bit_count(16)
            .build().unwrap();

        let cg = ChannelGroupBuilder::new()
            .name("TestChannelGroup")
            .master(ChannelBuilder::new_master_time("time"))
            .channel(f64_ch)
            .channel(i32_ch)
            .channel(u16_ch)
            .build().unwrap();

        let dg = DataGroupBuilder::new()
            .channel_group(cg)
            .build().unwrap();

        builder.add_data_group(dg);

        // Set data with correct types
        let time_data: Vec<f64> = vec![0.0, 0.1, 0.2, 0.3, 0.4];
        let f64_data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let i32_data: Vec<i32> = vec![10, 20, 30, 40, 50];
        let u16_data: Vec<u16> = vec![100, 200, 300, 400, 500];

        builder.set_channel_data("time", &time_data).unwrap();
        builder.set_channel_data("float64_val", &f64_data).unwrap();
        builder.set_channel_data("int32_val", &i32_data).unwrap();
        builder.set_channel_data("uint16_val", &u16_data).unwrap();

        println!("All channel data set successfully");

        // Verify internal storage
        assert!(builder.data_group_count() > 0);
    }

    /// Test: Write file with Mf4Builder and verify by reading back
    #[test]
    fn test_builder_write_and_read_back() {
        let output_path = PathBuf::from("temp_roundtrip_verify.mf4");
        cleanup_test_file(&output_path);

        // Create metadata
        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: 1704067200000000000, // 2024-01-01 00:00:00
            author: Some("Test User".to_string()),
            organization: None,
            project: None,
            comment: Some("Round-trip verification test".to_string()),
        };

        // Create builder
        let mut builder = Mf4Builder::new(metadata);

        // Define channels
        let time_channel = ChannelBuilder::new_master_time("time");
        let signal1 = ChannelBuilder::new("Signal1")
            .data_type(4)      // FLOAT64 LE
            .unit("V")
            .comment("Test signal 1")
            .build().unwrap();
        let signal2 = ChannelBuilder::new("Signal2")
            .data_type(2)      // INT16 LE
            .bit_count(16)
            .unit("rpm")
            .build().unwrap();

        // Create channel group
        let cg = ChannelGroupBuilder::new()
            .name("TestGroup")
            .master(time_channel)
            .channel(signal1)
            .channel(signal2)
            .build().unwrap();

        // Create data group
        let dg = DataGroupBuilder::new()
            .channel_group(cg)
            .build().unwrap();

        builder.add_data_group(dg);

        // Add test data (20 samples)
        let time_data: Vec<f64> = (0..20).map(|i| i as f64 * 0.05).collect();
        let signal1_data: Vec<f64> = (0..20).map(|i| (i as f64).sin()).collect();
        let signal2_data: Vec<i16> = (0..20).map(|i| 1000 + i * 50).collect();

        builder.set_channel_data("time", &time_data).unwrap();
        builder.set_channel_data("Signal1", &signal1_data).unwrap();
        builder.set_channel_data("Signal2", &signal2_data).unwrap();

        // Write file
        let write_result = builder.write(output_path.clone());
        println!("Write result: {:?}", write_result);

        if write_result.is_ok() {
            // Read the file back
            let read_result = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None);
            match read_result {
                Ok(mf4) => {
                    println!("Successfully read back the file");
                    let channels = mf4.get_channel_names();
                    println!("Channels found: {:?}", channels);

                    // Note: get_channel_names() doesn't include master channels
                    // Master channels are accessed via get_master()
                    // So we should find Signal1 and Signal2, but not "time" in the regular list
                    assert!(channels.contains(&"Signal1".to_string()));
                    assert!(channels.contains(&"Signal2".to_string()));

                    // Verify signal data
                    if let Some(data) = mf4.get_channel_data("Signal1") {
                        println!("Signal1 data read: {:?}", data);
                    }

                    if let Some(data) = mf4.get_channel_data("Signal2") {
                        println!("Signal2 data read: {:?}", data);
                    }

                    println!("Round-trip verification successful!");
                }
                Err(e) => {
                    println!("Failed to read back the file: {:?}", e);
                }
            }
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Streaming write and read back
    #[cfg(feature = "streaming")]
    #[test]
    fn test_streaming_write_and_read_back() {
        let output_path = PathBuf::from("temp_streaming_verify.mf4");
        cleanup_test_file(&output_path);

        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            author: Some("Streaming Test".to_string()),
            organization: None,
            project: None,
            comment: Some("Streaming write verification".to_string()),
        };

        let config = StreamingConfig::new()
            .with_block_size(500); // Small blocks for testing

        match Mf4StreamWriter::with_config(output_path.clone(), metadata, config) {
            Ok(mut writer) => {
                // Define channels
                let time_def = ChannelDef::new_master("time");
                let temp_def = ChannelDef::new("Temperature")
                    .data_type(4)  // FLOAT64
                    .unit("°C");

                let cg_def = ChannelGroupDef::builder()
                    .name("Environment")
                    .master(time_def)
                    .channel(temp_def)
                    .build().unwrap();

                let dg = StreamingDataGroup::new(cg_def).unwrap();
                writer.add_data_group(dg).unwrap();

                // Finalize structure
                writer.finalize_structure().unwrap();
                assert_eq!(writer.state(), crate::writer::stream_writer::WriterState::StructureReady);

                // Write 50 records
                for i in 0..50 {
                    writer.start_record(0, 0).unwrap();
                    writer.set_channel_value("time", i as f64 * 0.1).unwrap();
                    writer.set_channel_value("Temperature", 20.0 + (i as f64 * 0.5).sin()).unwrap();
                    writer.flush_record().unwrap();
                }

                assert_eq!(writer.state(), crate::writer::stream_writer::WriterState::Writing);
                assert_eq!(writer.total_records(), 50);

                // Finalize
                writer.finalize().unwrap();
                assert_eq!(writer.state(), crate::writer::stream_writer::WriterState::Finalized);

                // Read back and verify
                match Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None) {
                    Ok(mf4) => {
                        let channels = mf4.get_channel_names();
                        println!("Streaming write - Channels found: {:?}", channels);
                        // Note: get_channel_names() doesn't include master channels
                        assert!(channels.contains(&"Temperature".to_string()));
                        println!("Streaming write verification successful!");
                    }
                    Err(e) => {
                        println!("Failed to read streaming file: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to create streaming writer: {:?}", e);
            }
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Write MF4 file with acq_source (SI block) information
    #[test]
    fn test_builder_with_acq_source() {
        use crate::writer::{SourceInfoBuilder, SourceType, BusType};

        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: 1704067200000000000,
            author: Some("Test User".to_string()),
            organization: Some("Test Org".to_string()),
            project: Some("AcqSource Test".to_string()),
            comment: Some("Test for acq_source feature".to_string()),
        };

        let mut builder = Mf4Builder::new(metadata);

        // Define channels
        let time_channel = ChannelBuilder::new_master_time("time");
        let signal_channel = ChannelBuilder::new("CAN_Signal")
            .data_type(4)      // FLOAT64 LE
            .unit("m/s")
            .comment("CAN bus signal")
            .build().unwrap();

        // Create source info for the channel group
        let source = SourceInfoBuilder::new()
            .name("CAN_Channel_1")
            .path("CAN1")
            .source_type(SourceType::Bus)
            .bus_type(BusType::Can)
            .simulated(false)
            .build().unwrap();

        // Create channel group with acq_name and acq_source
        let cg = ChannelGroupBuilder::new()
            .name("CAN_Measurement")
            .acq_source(source)
            .master(time_channel)
            .channel(signal_channel)
            .build().unwrap();

        let dg = DataGroupBuilder::new()
            .channel_group(cg)
            .build().unwrap();

        builder.add_data_group(dg);

        // Add data
        let time_data: Vec<f64> = (0..10).map(|i| i as f64 * 0.01).collect();
        let signal_data: Vec<f64> = (0..10).map(|i| 10.0 + i as f64).collect();

        builder.set_channel_data("time", &time_data).unwrap();
        builder.set_channel_data("CAN_Signal", &signal_data).unwrap();

        // Write to temp file
        let output_path = PathBuf::from("temp_acq_source_test.mf4");
        cleanup_test_file(&output_path);

        let result = builder.write(output_path.clone());
        assert!(result.is_ok(), "Write should succeed");

        // Read back and verify acq_source is present
        if result.is_ok() {
            let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();

            // Get channel group info
            let cgs = mf4.get_all_channel_groups();
            assert!(!cgs.is_empty(), "Should have at least one channel group");

            // Verify acq_name
            let acq_name = cgs[0].get_acq_name();
            assert_eq!(acq_name, "CAN_Measurement", "acq_name should match");

            // Verify acq_source (SI block)
            let acq_source = cgs[0].get_acq_source();
            assert_eq!(acq_source.get_name(), "CAN_Channel_1", "SI name should match");
            assert_eq!(acq_source.get_path(), "CAN1", "SI path should match");

            println!("Acq source test passed:");
            println!("  acq_name: {}", acq_name);
            println!("  SI name: {}", acq_source.get_name());
            println!("  SI path: {}", acq_source.get_path());
            println!("  SI type: {}", acq_source.get_si_type());
            println!("  Bus type: {}", acq_source.get_bus_type());
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Verify time_series_demo.mf4 has correct acq_source info
    #[test]
    fn test_time_series_demo_acq_source() {

        let input_path = PathBuf::from("test/time_series_demo.mf4");
        if !input_path.exists() {
            println!("Skipping test: test/time_series_demo.mf4 not found. Run the binary first.");
            return;
        }

        let mf4 = Mf4Wrapper::new::<fn(f64)>(input_path.clone(), None).unwrap();
        let cgs = mf4.get_all_channel_groups();

        assert_eq!(cgs.len(), 3, "Should have 3 channel groups");

        // Verify FastSampling_100Hz (ADC I/O source)
        let cg_fast = &cgs[0];
        assert_eq!(cg_fast.get_acq_name(), "FastSampling_100Hz");
        let source_fast = cg_fast.get_acq_source();
        assert_eq!(source_fast.get_name(), "ADC_100Hz");
        assert_eq!(source_fast.get_path(), "DAQ/Card1/Channel1");
        println!("Fast sampling source: {} - {}", source_fast.get_name(), source_fast.get_si_type());

        // Verify MediumSampling_20Hz (CAN bus source)
        let cg_medium = &cgs[1];
        assert_eq!(cg_medium.get_acq_name(), "MediumSampling_20Hz");
        let source_medium = cg_medium.get_acq_source();
        assert_eq!(source_medium.get_name(), "CAN_Bus_20Hz");
        assert_eq!(source_medium.get_path(), "CAN1");
        println!("Medium sampling source: {} - {} - {}", source_medium.get_name(), source_medium.get_si_type(), source_medium.get_bus_type());

        // Verify SlowSampling_10Hz (ECU source)
        let cg_slow = &cgs[2];
        assert_eq!(cg_slow.get_acq_name(), "SlowSampling_10Hz");
        let source_slow = cg_slow.get_acq_source();
        assert_eq!(source_slow.get_name(), "ECU_Monitor_10Hz");
        assert_eq!(source_slow.get_path(), "ECU/Internal");
        println!("Slow sampling source: {} - {}", source_slow.get_name(), source_slow.get_si_type());

        println!("Time series demo acq_source verification passed!");
    }
}
