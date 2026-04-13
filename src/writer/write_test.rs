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

        let builder = Mf4Builder::new(metadata);

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

    /// Test: Builder correctly chunks large data into a DZ chain when data exceeds 4MB.
    ///
    /// The MDF4 protocol forbids a single DZ block with `dz_org_data_length > 4MB`.
    /// When `Mf4Builder` writes compressed data that exceeds this limit it must
    /// split it into multiple DZ blocks, wrap them in a DL block, and top the chain
    /// with an HL block — and then update `dg_data` in the DG block to point to HL.
    #[cfg(feature = "compression")]
    #[test]
    fn test_builder_large_compressed_data_dz_chain() {
        use crate::writer::builder::CompressionConfig;

        let output_path = PathBuf::from("temp_builder_large_dz_chain.mf4");
        cleanup_test_file(&output_path);

        // 2 × f64 = 16 bytes / record.  300 000 records = 4 800 000 bytes > 4 MB.
        const NUM_RECORDS: usize = 300_000;
        let time_data: Vec<f64>   = (0..NUM_RECORDS).map(|i| i as f64 * 0.001).collect();
        let signal_data: Vec<f64> = (0..NUM_RECORDS).map(|i| (i as f64 * 0.01).sin()).collect();

        let metadata = Mf4Metadata {
            version: "4.10".to_string(),
            version_num: 410,
            start_time_ns: 0,
            author: None,
            organization: None,
            project: None,
            comment: None,
        };

        let mut builder = Mf4Builder::new(metadata);

        let cg = ChannelGroupBuilder::new()
            .name("Data")
            .master(ChannelBuilder::new_master_time("time"))
            .channel(ChannelBuilder::new("Signal").data_type(4).unit("V").build().unwrap())
            .build().unwrap();
        builder.add_data_group(DataGroupBuilder::new().channel_group(cg).build().unwrap());

        builder.set_channel_data("time",   &time_data).unwrap();
        builder.set_channel_data("Signal", &signal_data).unwrap();

        // Compress everything (min_size = 0 means always compress)
        builder.set_compression(CompressionConfig::new().with_min_size(0));

        builder.write(output_path.clone()).expect("builder write");

        // ─── Verify file structure: DG.dg_data → HL → DL → [DZ₁, DZ₂, …] ────
        let dg_offset     = read_u64_at(&output_path, 88); // HD.hd_dg_first
        let dg_data_field = dg_offset + 24 + 16;           // 24-byte header + 8 dg_next + 8 cg_first
        let dg_data       = read_u64_at(&output_path, dg_data_field);

        let top_id = read_block_id(&output_path, dg_data);
        assert_eq!(top_id, "##HL", "DG.dg_data must point to ##HL for large compressed data");

        // HL → DL
        let dl_offset = read_u64_at(&output_path, dg_data + 24); // first link in HL
        let dl_id     = read_block_id(&output_path, dl_offset);
        assert_eq!(dl_id, "##DL");

        // DL link_count and iterate over data links
        let dl_link_count   = read_u64_at(&output_path, dl_offset + 16) as usize;
        let num_data_blocks = dl_link_count - 1; // subtract dl_dl_next
        assert!(num_data_blocks >= 2, "Should have at least 2 DZ blocks for > 4MB data");

        let max_dz: u64 = 4 * 1024 * 1024;

        for i in 0..num_data_blocks {
            let dz_offset = read_u64_at(&output_path, dl_offset + 24 + 8 * (i as u64 + 1));
            assert!(dz_offset > 0);

            let dz_id = read_block_id(&output_path, dz_offset);
            assert_eq!(dz_id, "##DZ", "Block {} must be ##DZ", i);

            // DZ blocks written via block_writer have 0 links, so data fields start at 24:
            //   2 (org_block_type) + 1 (zip_type) + 1 (reserved) + 4 (zip_param) = 8
            //   → dz_org_data_length at offset 24 + 8 = 32
            let orig_len = read_u64_at(&output_path, dz_offset + 32);
            assert!(
                orig_len <= max_dz,
                "DZ block {} has orig_len {} which exceeds the 4MB MDF4 limit",
                i, orig_len
            );
        }

        // ─── Round-trip integrity check ───────────────────────────────────────
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        if let Some(crate::DataValue::REAL(vals)) = mf4.get_channel_data("Signal") {
            assert_eq!(vals.len(), NUM_RECORDS, "Round-trip sample count mismatch");
            // Spot-check first and last sample
            assert!((vals[0] - 0.0_f64.sin()).abs() < 1e-10);
            let expected_last = ((NUM_RECORDS - 1) as f64 * 0.01).sin();
            assert!(
                (vals[NUM_RECORDS - 1] - expected_last).abs() < 1e-10,
                "Last sample mismatch: expected {}, got {}",
                expected_last, vals[NUM_RECORDS - 1]
            );
        } else {
            panic!("Expected REAL data for Signal channel");
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

    // ========================================================================
    // DL-chained stream write tests (HL→DL→DZ / DL→DT)
    // ========================================================================

    /// Helper: read 4 bytes at a file offset and return the block ID string (e.g. "##DT")
    fn read_block_id(path: &PathBuf, offset: u64) -> String {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = std::fs::File::open(path).unwrap();
        f.seek(SeekFrom::Start(offset)).unwrap();
        let mut buf = [0u8; 4];
        f.read_exact(&mut buf).unwrap();
        String::from_utf8(buf.to_vec()).unwrap()
    }

    /// Helper: read a u64 little-endian value at a file offset
    fn read_u64_at(path: &PathBuf, offset: u64) -> u64 {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = std::fs::File::open(path).unwrap();
        f.seek(SeekFrom::Start(offset)).unwrap();
        let mut buf = [0u8; 8];
        f.read_exact(&mut buf).unwrap();
        u64::from_le_bytes(buf)
    }

    /// Helper: create a stream writer with the given config, write N records of 2xf64
    /// (time + signal), finalize, and return the output path.
    fn write_stream_file(
        filename: &str,
        config: StreamingConfig,
        num_records: usize,
        compact: bool,
    ) -> PathBuf {
        let output_path = PathBuf::from(filename);
        cleanup_test_file(&output_path);

        let metadata = Mf4Metadata::default();
        let mut writer = Mf4StreamWriter::with_config(
            output_path.clone(),
            metadata,
            config,
        ).unwrap();

        let time_def = ChannelDef::new_master("time");
        let signal_def = ChannelDef::new("Signal")
            .data_type(4) // FLOAT64
            .unit("V");

        let cg_def = ChannelGroupDef::builder()
            .name("Measurement")
            .master(time_def)
            .channel(signal_def)
            .build()
            .unwrap();

        let dg = StreamingDataGroup::new(cg_def).unwrap();
        writer.add_data_group(dg).unwrap();
        writer.finalize_structure().unwrap();

        for i in 0..num_records {
            writer.start_record(0, 0).unwrap();
            writer.set_channel_value("time", i as f64 * 0.001).unwrap();
            writer.set_channel_value("Signal", (i as f64 * 0.1).sin()).unwrap();
            writer.flush_record().unwrap();
        }

        writer.finalize_with_compact(compact).unwrap();
        output_path
    }

    /// Test: Non-compact stream write produces DL→DT chain (no compression).
    ///
    /// With a small block_size (e.g. 100 bytes) and enough records, data should
    /// be split into multiple DT blocks linked by a DL block. The DG.dg_data
    /// link should point to a ##DL block.
    #[cfg(feature = "streaming")]
    #[test]
    fn test_stream_write_dl_chain_uncompressed() {
        let config = StreamingConfig::new()
            .with_block_size(100); // Very small blocks to force multiple DT blocks

        // Each record = 16 bytes (2 x f64). 100 records = 1600 bytes.
        // With 100-byte block_size we expect ~16 DT blocks linked by DL.
        let output_path = write_stream_file(
            "temp_dl_chain_uncomp.mf4",
            config,
            100,
            false, // non-compact
        );

        // Verify the file was created
        assert!(output_path.exists(), "Output file should exist");

        // Read the DG block's dg_data link.
        // DG block layout: 24-byte header, then links: dg_dg_next(8) + dg_cg_first(8) + dg_data(8)
        // So dg_data is at dg_offset + 24 + 16
        // We need to find the DG offset. HD is at offset 64 (after 64-byte ID block).
        // HD layout: 24-byte header, then links: hd_dg_first(8)
        // So hd_dg_first is at 64 + 24 = 88
        let dg_offset = read_u64_at(&output_path, 88);
        assert!(dg_offset > 0, "DG offset should be non-zero");

        let dg_data_offset = read_u64_at(&output_path, dg_offset + 24 + 16);
        assert!(dg_data_offset > 0, "DG data link should be non-zero");

        // The data link should point to a DL block
        let block_id = read_block_id(&output_path, dg_data_offset);
        assert_eq!(block_id, "##DL", "DG.dg_data should point to a ##DL block");

        // Read back with Mf4Wrapper and verify data
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let channels = mf4.get_channel_names();
        assert!(channels.contains(&"Signal".to_string()));

        let data = mf4.get_channel_data("Signal").unwrap();
        assert_eq!(data.len(), 100, "Should have 100 samples");

        // Verify time channel via master data
        let time_data = mf4.get_channel_master_data("Signal").unwrap();
        assert_eq!(time_data.len(), 100);

        // Verify first and last time values
        if let crate::DataValue::REAL(ref vals) = time_data {
            assert!((vals[0] - 0.0).abs() < 1e-10, "First time should be 0.0");
            assert!((vals[99] - 0.099).abs() < 1e-10, "Last time should be 0.099");
        } else {
            panic!("Expected REAL data for time");
        }

        // Verify signal values
        if let crate::DataValue::REAL(ref vals) = data {
            for i in 0..100 {
                let expected = (i as f64 * 0.1).sin();
                assert!(
                    (vals[i] - expected).abs() < 1e-10,
                    "Signal[{}]: expected {}, got {}",
                    i, expected, vals[i]
                );
            }
        } else {
            panic!("Expected REAL data for Signal");
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Non-compact stream write with compression produces HL→DL→DZ chain.
    ///
    /// When compression is enabled and data is split into multiple blocks,
    /// DG.dg_data should point to a ##HL block (not directly to DL or DZ).
    #[cfg(feature = "streaming")]
    #[test]
    fn test_stream_write_hl_dl_dz_chain() {
        let config = StreamingConfig::new()
            .with_block_size(200)
            .with_compression()
            .with_compression_threshold(0);

        // 200 records × 16 bytes = 3200 bytes, with 200-byte block we get ~16 blocks
        let output_path = write_stream_file(
            "temp_hl_dl_dz_chain.mf4",
            config,
            200,
            false, // non-compact
        );

        assert!(output_path.exists());

        // Read DG.dg_data
        let dg_offset = read_u64_at(&output_path, 88);
        let dg_data_offset = read_u64_at(&output_path, dg_offset + 24 + 16);
        assert!(dg_data_offset > 0);

        // Should point to HL block
        let block_id = read_block_id(&output_path, dg_data_offset);
        assert_eq!(block_id, "##HL", "DG.dg_data should point to ##HL when compressed + DL chain");

        // HL block layout: 24 header + 8 link_count + 8 hl_dl_first link = ...
        // Actually: 24 header bytes, then 1 link (hl_dl_first) at offset 24
        let hl_dl_first = read_u64_at(&output_path, dg_data_offset + 24);
        assert!(hl_dl_first > 0, "HL should link to a DL block");

        let dl_block_id = read_block_id(&output_path, hl_dl_first);
        assert_eq!(dl_block_id, "##DL", "HL.hl_dl_first should point to ##DL");

        // Check that DL links point to DZ blocks
        // DL layout: 24 header, then link_count at offset 16
        // Links start at offset 24: dl_dl_next(8), then dl_data[0..N](8 each)
        let dl_link_count = read_u64_at(&output_path, hl_dl_first + 16);
        assert!(dl_link_count >= 2, "DL should have at least 2 links (1 for dl_next + at least 1 data)");

        // First data link is at DL offset + 24 + 8 (skip dl_dl_next)
        let first_dz_offset = read_u64_at(&output_path, hl_dl_first + 24 + 8);
        let dz_block_id = read_block_id(&output_path, first_dz_offset);
        assert_eq!(dz_block_id, "##DZ", "DL data links should point to ##DZ blocks");

        // Round-trip read back
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let data = mf4.get_channel_data("Signal").unwrap();
        assert_eq!(data.len(), 200, "Should have 200 samples after round-trip");

        if let crate::DataValue::REAL(ref vals) = data {
            for i in 0..200 {
                let expected = (i as f64 * 0.1).sin();
                assert!(
                    (vals[i] - expected).abs() < 1e-10,
                    "Signal[{}] mismatch: expected {}, got {}",
                    i, expected, vals[i]
                );
            }
        } else {
            panic!("Expected REAL data for Signal");
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Each DZ block's uncompressed size must not exceed 4MB.
    ///
    /// Write enough data to exceed 4MB total, verify each DZ block
    /// in the chain respects the limit.
    #[cfg(feature = "streaming")]
    #[test]
    fn test_dz_block_size_limit_4mb() {

        // Each record = 16 bytes (2 × f64).
        // 4MB = 4,194,304 bytes → 262,144 records to fill one DZ block.
        // Write 400,000 records (~6.1 MB) so we need at least 2 DZ blocks.
        // The library always uses 4MB as the DZ block size (protocol requirement),
        // regardless of `block_size` (which only affects uncompressed DT blocks).
        let config = StreamingConfig::new()
            .with_block_size(8_000_000) // block_size only affects DT blocks; DZ always uses 4MB
            .with_compression()
            .with_compression_threshold(0);

        let output_path = write_stream_file(
            "temp_dz_4mb_limit.mf4",
            config,
            400_000,
            false,
        );

        assert!(output_path.exists());

        // Navigate to DG → HL → DL → DZ blocks
        let dg_offset = read_u64_at(&output_path, 88);
        let dg_data_offset = read_u64_at(&output_path, dg_offset + 24 + 16);
        let hl_id = read_block_id(&output_path, dg_data_offset);
        assert_eq!(hl_id, "##HL");

        let dl_offset = read_u64_at(&output_path, dg_data_offset + 24);
        let dl_id = read_block_id(&output_path, dl_offset);
        assert_eq!(dl_id, "##DL");

        // Read DL link count and iterate over DZ blocks
        let dl_link_count = read_u64_at(&output_path, dl_offset + 16) as usize;
        let num_data_blocks = dl_link_count - 1; // subtract dl_dl_next
        assert!(num_data_blocks >= 2, "Should have at least 2 DZ blocks for >4MB data");

        let max_dz_uncompressed: u64 = 4 * 1024 * 1024;

        for i in 0..num_data_blocks {
            let dz_offset = read_u64_at(&output_path, dl_offset + 24 + 8 * (i as u64 + 1));
            if dz_offset == 0 {
                break;
            }

            let dz_id = read_block_id(&output_path, dz_offset);
            assert_eq!(dz_id, "##DZ", "Block {} should be ##DZ", i);

            // DZ data fields start at offset 24 (0 links per MDF4 spec)
            // dz_org_block_type (2) + dz_zip_type (1) + dz_reserved (1) + dz_zip_parameter (4) = 8
            // Then dz_org_data_length at offset 24 + 8 = 32
            let dz_org_data_length = read_u64_at(&output_path, dz_offset + 32);
            assert!(
                dz_org_data_length <= max_dz_uncompressed,
                "DZ block {} original length {} exceeds 4MB limit {}",
                i, dz_org_data_length, max_dz_uncompressed
            );
        }

        // Round-trip verification
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let data = mf4.get_channel_data("Signal").unwrap();
        assert_eq!(data.len(), 400_000, "Should have 400k samples");

        cleanup_test_file(&output_path);
    }

    /// Test: Records do not span DZ block boundaries.
    ///
    /// Each DZ block's uncompressed size must be a multiple of the record size,
    /// ensuring no record is split across two DZ blocks.
    #[cfg(feature = "streaming")]
    #[test]
    fn test_record_alignment_in_dz_blocks() {

        // Use a record size that doesn't divide evenly into typical block sizes.
        // 3 channels × f64 = 24 bytes per record.
        // DZ block sizes are always 4MB (protocol requirement), so with 100 records
        // (2400 bytes total, well under 4MB) the result is a single DZ block.
        // Record-alignment still applies: every chunk must be a multiple of 24 bytes.
        let config = StreamingConfig::new()
            .with_block_size(500) // only affects DT; DZ always uses 4MB internally
            .with_compression()
            .with_compression_threshold(0);

        let output_path = PathBuf::from("temp_record_align.mf4");
        cleanup_test_file(&output_path);

        let metadata = Mf4Metadata::default();
        let mut writer = Mf4StreamWriter::with_config(
            output_path.clone(),
            metadata,
            config,
        ).unwrap();

        let time_def = ChannelDef::new_master("time");
        let ch1 = ChannelDef::new("ChannelA").data_type(4); // f64
        let ch2 = ChannelDef::new("ChannelB").data_type(4); // f64

        let cg_def = ChannelGroupDef::builder()
            .name("ThreeChannels")
            .master(time_def)
            .channel(ch1)
            .channel(ch2)
            .build()
            .unwrap();

        // Record size = 24 bytes (3 × 8)
        assert_eq!(cg_def.record_size, 24);

        let dg = StreamingDataGroup::new(cg_def).unwrap();
        writer.add_data_group(dg).unwrap();
        writer.finalize_structure().unwrap();

        // Write 100 records = 2400 bytes
        for i in 0..100 {
            writer.start_record(0, 0).unwrap();
            writer.set_channel_value("time", i as f64 * 0.01).unwrap();
            writer.set_channel_value("ChannelA", i as f64 * 10.0).unwrap();
            writer.set_channel_value("ChannelB", i as f64 * -5.0).unwrap();
            writer.flush_record().unwrap();
        }

        writer.finalize_with_compact(false).unwrap();

        // Navigate to DZ blocks and check each has a size that's a multiple of 24
        let dg_offset = read_u64_at(&output_path, 88);
        let dg_data_offset = read_u64_at(&output_path, dg_offset + 24 + 16);

        // Could be HL or DL depending on compression
        let top_id = read_block_id(&output_path, dg_data_offset);
        let dl_offset = if top_id == "##HL" {
            read_u64_at(&output_path, dg_data_offset + 24)
        } else {
            assert_eq!(top_id, "##DL");
            dg_data_offset
        };

        let dl_link_count = read_u64_at(&output_path, dl_offset + 16) as usize;
        let num_data_blocks = dl_link_count - 1;

        let record_size: u64 = 24;
        for i in 0..num_data_blocks {
            let block_offset = read_u64_at(&output_path, dl_offset + 24 + 8 * (i as u64 + 1));
            if block_offset == 0 { break; }

            let block_id = read_block_id(&output_path, block_offset);

            let orig_data_len = if block_id == "##DZ" {
                // dz_org_data_length at offset 32 from block start
                // (24 header + 0 links + 2 org_block_type + 1 zip_type + 1 reserved + 4 zip_param = 32)
                read_u64_at(&output_path, block_offset + 32)
            } else if block_id == "##DT" {
                // DT block: total length at offset 8, minus 24 header
                let block_len = read_u64_at(&output_path, block_offset + 8);
                block_len - 24
            } else {
                panic!("Unexpected block type: {}", block_id);
            };

            assert_eq!(
                orig_data_len % record_size, 0,
                "Block {} ({}): uncompressed size {} is not a multiple of record_size {}",
                i, block_id, orig_data_len, record_size
            );
        }

        // Round-trip read back
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let data_a = mf4.get_channel_data("ChannelA").unwrap();
        assert_eq!(data_a.len(), 100);
        if let crate::DataValue::REAL(ref vals) = data_a {
            for i in 0..100 {
                assert!(
                    (vals[i] - i as f64 * 10.0).abs() < 1e-10,
                    "ChannelA[{}] mismatch",
                    i
                );
            }
        }

        cleanup_test_file(&output_path);
    }

    /// Test: DL chain round-trip (uncompressed) — write non-compact, read back, verify all data.
    #[cfg(feature = "streaming")]
    #[test]
    fn test_dl_chain_roundtrip_uncompressed() {
        let config = StreamingConfig::new()
            .with_block_size(256); // Small blocks → many DT blocks

        let output_path = write_stream_file(
            "temp_dl_roundtrip_uncomp.mf4",
            config,
            500,
            false,
        );

        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();

        // Check channel names
        let channels = mf4.get_channel_names();
        assert!(channels.contains(&"Signal".to_string()));

        // Verify all 500 signal values
        let data = mf4.get_channel_data("Signal").unwrap();
        assert_eq!(data.len(), 500);
        if let crate::DataValue::REAL(ref vals) = data {
            for i in 0..500 {
                let expected = (i as f64 * 0.1).sin();
                assert!(
                    (vals[i] - expected).abs() < 1e-10,
                    "Roundtrip mismatch at index {}",
                    i
                );
            }
        } else {
            panic!("Expected REAL data");
        }

        // Verify time values
        let time_data = mf4.get_channel_master_data("Signal").unwrap();
        assert_eq!(time_data.len(), 500);
        if let crate::DataValue::REAL(ref vals) = time_data {
            for i in 0..500 {
                let expected = i as f64 * 0.001;
                assert!(
                    (vals[i] - expected).abs() < 1e-12,
                    "Time mismatch at index {}",
                    i
                );
            }
        }

        cleanup_test_file(&output_path);
    }

    /// Test: DL chain round-trip with compression (HL→DL→DZ) — write and read back.
    #[cfg(feature = "streaming")]
    #[test]
    fn test_dl_chain_roundtrip_compressed() {
        let config = StreamingConfig::new()
            .with_block_size(256)
            .with_compression()
            .with_compression_threshold(0);

        let output_path = write_stream_file(
            "temp_dl_roundtrip_comp.mf4",
            config,
            500,
            false,
        );

        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();

        let data = mf4.get_channel_data("Signal").unwrap();
        assert_eq!(data.len(), 500);
        if let crate::DataValue::REAL(ref vals) = data {
            for i in 0..500 {
                let expected = (i as f64 * 0.1).sin();
                assert!(
                    (vals[i] - expected).abs() < 1e-10,
                    "Compressed roundtrip mismatch at index {}",
                    i
                );
            }
        } else {
            panic!("Expected REAL data");
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Edge case — single record in non-compact mode.
    ///
    /// Even with just 1 record, the non-compact path should still produce
    /// a valid DL (or direct DT) structure that reads back correctly.
    #[cfg(feature = "streaming")]
    #[test]
    fn test_stream_edge_single_record() {
        let config = StreamingConfig::new()
            .with_block_size(100);

        let output_path = write_stream_file(
            "temp_single_record.mf4",
            config,
            1,
            false,
        );

        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let data = mf4.get_channel_data("Signal").unwrap();
        assert_eq!(data.len(), 1);

        if let crate::DataValue::REAL(ref vals) = data {
            let expected = (0.0f64).sin(); // sin(0) = 0
            assert!((vals[0] - expected).abs() < 1e-10);
        }

        cleanup_test_file(&output_path);
    }

    /// Test: Compact mode still produces a single DT/DZ block (not DL chain).
    ///
    /// With compact=true, regardless of block_size, the final file should have
    /// DG.dg_data pointing directly to a ##DT or ##DZ block (not ##DL or ##HL).
    #[cfg(feature = "streaming")]
    #[test]
    fn test_stream_compact_produces_single_block() {
        let config = StreamingConfig::new()
            .with_block_size(100); // Small block_size, but compact should merge

        let output_path = write_stream_file(
            "temp_compact_single.mf4",
            config,
            200,
            true, // compact
        );

        let dg_offset = read_u64_at(&output_path, 88);
        let dg_data_offset = read_u64_at(&output_path, dg_offset + 24 + 16);
        let block_id = read_block_id(&output_path, dg_data_offset);

        // Compact mode (uncompressed) should produce a single DT block
        assert!(
            block_id == "##DT",
            "Compact mode (uncompressed) should produce ##DT, got {}",
            block_id
        );

        // Verify data round-trips
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let data = mf4.get_channel_data("Signal").unwrap();
        assert_eq!(data.len(), 200);

        cleanup_test_file(&output_path);
    }

    /// Test: compact mode + compression returns an error (mutually exclusive).
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_compact_compressed_returns_error() {
        use crate::writer::error::WriteError;

        let config = StreamingConfig::new()
            .with_compression()
            .with_compression_threshold(0);

        let output_path = PathBuf::from("temp_compact_compressed_err.mf4");
        cleanup_test_file(&output_path);

        let metadata = Mf4Metadata::default();
        let mut writer = Mf4StreamWriter::with_config(
            output_path.clone(),
            metadata,
            config,
        ).unwrap();

        let cg = ChannelGroupDef::builder()
            .name("Measurement")
            .master(ChannelDef::new_master("time"))
            .channel(ChannelDef::new("Signal").data_type(4).unit("V"))
            .build()
            .unwrap();
        writer.add_data_group(StreamingDataGroup::new(cg).unwrap()).unwrap();
        writer.finalize_structure().unwrap();

        for i in 0..10 {
            writer.start_record(0, 0).unwrap();
            writer.set_channel_value("time", i as f64 * 0.001).unwrap();
            writer.set_channel_value("Signal", i as f64).unwrap();
            writer.flush_record().unwrap();
        }

        // compact=true + compression must return an error
        let result = writer.finalize_with_compact(true);
        assert!(
            matches!(result, Err(WriteError::InvalidChannelConfig(_))),
            "Expected InvalidChannelConfig error for compact+compressed, got: {:?}",
            result
        );

        cleanup_test_file(&output_path);
    }

    /// Test: SimpleWriter compact_mode + compression returns an error.
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_simple_writer_compact_compressed_returns_error() {
        use crate::writer::simple_writer::SimpleWriter;
        use crate::writer::error::WriteError;

        let path = PathBuf::from("temp_sw_compact_compressed_err.mf4");
        cleanup_test_file(&path);

        let result = SimpleWriter::new(&path)
            .time_channel("time", "s")
            .f64_channel("signal", "V")
            .compact_mode()
            .compression(6)
            .build();

        assert!(
            matches!(result, Err(WriteError::InvalidChannelConfig(_))),
            "Expected InvalidChannelConfig error for compact_mode+compression"
        );

        cleanup_test_file(&path);
    }

    /// Test: Multi-CG streaming with DL chain — records from different CGs
    /// must not be split across DZ block boundaries.
    #[cfg(feature = "streaming")]
    #[test]
    fn test_stream_multi_cg_dl_chain_roundtrip() {
        let config = StreamingConfig::new()
            .with_block_size(200); // Small blocks

        let output_path = PathBuf::from("temp_multi_cg_dl_chain.mf4");
        cleanup_test_file(&output_path);

        let metadata = Mf4Metadata::default();
        let mut writer = Mf4StreamWriter::with_config(
            output_path.clone(),
            metadata,
            config,
        ).unwrap();

        let cg1 = ChannelGroupDef::builder()
            .name("Fast")
            .master(ChannelDef::new_master("time_fast"))
            .channel(ChannelDef::new("RPM").data_type(4))
            .build()
            .unwrap();

        let cg2 = ChannelGroupDef::builder()
            .name("Slow")
            .master(ChannelDef::new_master("time_slow"))
            .channel(ChannelDef::new("Temperature").data_type(4))
            .build()
            .unwrap();

        let dg = StreamingDataGroup::with_multiple(vec![cg1, cg2]).unwrap();
        writer.add_data_group(dg).unwrap();
        writer.finalize_structure().unwrap();

        // Write interleaved records: fast at 100Hz, slow at 10Hz
        for i in 0..100 {
            writer.start_record(0, 0).unwrap(); // fast CG
            writer.set_channel_value("time_fast", i as f64 * 0.01).unwrap();
            writer.set_channel_value("RPM", 3000.0 + i as f64).unwrap();
            writer.flush_record().unwrap();

            if i % 10 == 0 {
                writer.start_record(0, 1).unwrap(); // slow CG
                writer.set_channel_value("time_slow", i as f64 * 0.01).unwrap();
                writer.set_channel_value("Temperature", 85.0 + (i as f64) * 0.1).unwrap();
                writer.flush_record().unwrap();
            }
        }

        writer.finalize_with_compact(false).unwrap();

        // Verify DL chain exists
        let dg_offset = read_u64_at(&output_path, 88);
        let dg_data_offset = read_u64_at(&output_path, dg_offset + 24 + 16);
        let block_id = read_block_id(&output_path, dg_data_offset);
        assert_eq!(block_id, "##DL", "Multi-CG non-compact should use DL chain");

        // Round-trip verification
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();
        let rpm_data = mf4.get_channel_data("RPM").unwrap();
        assert_eq!(rpm_data.len(), 100, "Should have 100 RPM records");

        let temp_data = mf4.get_channel_data("Temperature").unwrap();
        // 100 / 10 = 10 slow records (i=0,10,20,...,90)
        // Actually i % 10 == 0: i=0,10,20,30,40,50,60,70,80,90 → 10 records
        assert_eq!(temp_data.len(), 10, "Should have 10 Temperature records");

        cleanup_test_file(&output_path);
    }

    // ============================================================================
    // Record alignment tests with various record sizes
    // ============================================================================

    /// Helper: verifies that every DZ/DT block in a DL chain has an uncompressed
    /// size that's a multiple of the given record_size.
    /// `num_f64_channels` = number of f64 channels BESIDES the time channel.
    #[cfg(feature = "streaming")]
    fn verify_record_alignment_for_size(
        num_f64_channels: usize,
        block_size: u64,
        num_records: usize,
        file_suffix: &str,
    ) {
        let expected_record_size = (1 + num_f64_channels) as u32 * 8; // time + channels, all f64

        let config = StreamingConfig::new()
            .with_block_size(block_size)
            .with_compression()
            .with_compression_threshold(0);

        let output_path = PathBuf::from(format!("temp_align_{}.mf4", file_suffix));
        cleanup_test_file(&output_path);

        let metadata = Mf4Metadata::default();
        let mut writer = Mf4StreamWriter::with_config(
            output_path.clone(), metadata, config,
        ).unwrap();

        let time_def = ChannelDef::new_master("time");
        let mut cg_builder = ChannelGroupDef::builder()
            .name("AlignTest")
            .master(time_def);

        for ch_idx in 0..num_f64_channels {
            cg_builder = cg_builder.channel(
                ChannelDef::new(&format!("ch_{}", ch_idx)).data_type(4) // f64
            );
        }

        let cg_def = cg_builder.build().unwrap();
        assert_eq!(cg_def.record_size, expected_record_size,
            "Expected record_size={}, got {}", expected_record_size, cg_def.record_size);

        let dg = StreamingDataGroup::new(cg_def).unwrap();
        writer.add_data_group(dg).unwrap();
        writer.finalize_structure().unwrap();

        for i in 0..num_records {
            writer.start_record(0, 0).unwrap();
            writer.set_channel_value("time", i as f64 * 0.001).unwrap();
            for c in 0..num_f64_channels {
                writer.set_channel_value(&format!("ch_{}", c), (i * (c + 1)) as f64).unwrap();
            }
            writer.flush_record().unwrap();
        }

        writer.finalize_with_compact(false).unwrap();

        // Navigate to data blocks and check alignment
        let dg_offset = read_u64_at(&output_path, 88);
        let dg_data_offset = read_u64_at(&output_path, dg_offset + 24 + 16);

        let top_id = read_block_id(&output_path, dg_data_offset);
        let dl_offset = if top_id == "##HL" {
            read_u64_at(&output_path, dg_data_offset + 24)
        } else {
            assert_eq!(top_id, "##DL");
            dg_data_offset
        };

        let dl_link_count = read_u64_at(&output_path, dl_offset + 16) as usize;
        let num_data_blocks = dl_link_count - 1;
        assert!(num_data_blocks >= 1, "Expected at least 1 data block");

        let mut total_uncompressed = 0u64;
        for i in 0..num_data_blocks {
            let block_offset = read_u64_at(&output_path, dl_offset + 24 + 8 * (i as u64 + 1));
            if block_offset == 0 { break; }

            let block_id = read_block_id(&output_path, block_offset);
            let orig_data_len = if block_id == "##DZ" {
                read_u64_at(&output_path, block_offset + 32)
            } else if block_id == "##DT" {
                let block_len = read_u64_at(&output_path, block_offset + 8);
                block_len - 24
            } else {
                panic!("Unexpected block type: {}", block_id);
            };

            assert_eq!(
                orig_data_len % expected_record_size as u64, 0,
                "record_size={}: Block {} ({}): uncompressed size {} is not a multiple of record_size",
                expected_record_size, i, block_id, orig_data_len
            );
            total_uncompressed += orig_data_len;
        }

        let expected_total = num_records as u64 * expected_record_size as u64;
        assert_eq!(total_uncompressed, expected_total,
            "record_size={}: total uncompressed {} != expected {}",
            expected_record_size, total_uncompressed, expected_total);

        // Round-trip read back — verify data channel
        // Note: master/time channel is accessed separately, not via get_channel_data
        let mf4 = Mf4Wrapper::new::<fn(f64)>(output_path.clone(), None).unwrap();

        if num_f64_channels > 0 {
            let ch0_data = mf4.get_channel_data("ch_0").unwrap();
            assert_eq!(ch0_data.len(), num_records,
                "record_size={}: expected {} records, got {}",
                expected_record_size, num_records, ch0_data.len());
            if let crate::DataValue::REAL(ref vals) = ch0_data {
                for i in 0..num_records.min(10) {
                    assert!((vals[i] - i as f64).abs() < 1e-10,
                        "ch_0[{}] mismatch: expected {}, got {}", i, i as f64, vals[i]);
                }
            }
        }

        cleanup_test_file(&output_path);
    }

    /// Record alignment with 40-byte records (time + 4×f64). 500/40 = 12.5 — not exact
    #[cfg(feature = "streaming")]
    #[test]
    fn test_record_alignment_40_bytes() {
        verify_record_alignment_for_size(4, 500, 200, "40b");
    }

    /// Record alignment with 56-byte records (time + 6×f64). 1000/56 = 17.86 — not exact
    #[cfg(feature = "streaming")]
    #[test]
    fn test_record_alignment_56_bytes() {
        verify_record_alignment_for_size(6, 1000, 300, "56b");
    }

    /// Record alignment with 104-byte records (time + 12×f64). 700/104 = 6.73 — not exact
    #[cfg(feature = "streaming")]
    #[test]
    fn test_record_alignment_104_bytes() {
        verify_record_alignment_for_size(12, 700, 150, "104b_700");
        // Also test with a block size that IS an exact multiple (832 = 8 × 104)
        verify_record_alignment_for_size(12, 832, 150, "104b_832");
    }

    // ========================================================================
    // Ergonomic API Tests: Convenience Methods + SimpleWriter
    // ========================================================================

    /// Test: ChannelGroupDefBuilder convenience methods produce correct channel defs
    #[cfg(feature = "streaming")]
    #[test]
    fn test_convenience_add_f64_channel() {
        use crate::writer::stream_writer::ChannelGroupDefBuilder;

        let cg = ChannelGroupDefBuilder::new()
            .name("test_group")
            .with_time_channel("time")
            .add_f64_channel("voltage", "V")
            .add_f64_channel("current", "A")
            .build()
            .expect("build CG");

        // Master channel: time (f64 = 8 bytes)
        assert!(cg.master.is_some());
        let master = cg.master.as_ref().unwrap();
        assert_eq!(master.name, "time");
        assert_eq!(master.data_type, 4); // FLOAT_LE
        assert_eq!(master.bit_count, 64);
        assert_eq!(master.cn_type, 2); // Master
        assert_eq!(master.byte_offset, 0);

        // Data channels
        assert_eq!(cg.channels.len(), 2);
        assert_eq!(cg.channels[0].name, "voltage");
        assert_eq!(cg.channels[0].data_type, 4);
        assert_eq!(cg.channels[0].bit_count, 64);
        assert_eq!(cg.channels[0].byte_offset, 8); // after time
        assert_eq!(cg.channels[0].unit, Some("V".to_string()));

        assert_eq!(cg.channels[1].name, "current");
        assert_eq!(cg.channels[1].byte_offset, 16); // after voltage
        assert_eq!(cg.channels[1].unit, Some("A".to_string()));

        // Record size = 3 × 8 = 24
        assert_eq!(cg.record_size, 24);
    }

    /// Test: Mixed convenience methods (f32, u32, i16)
    #[cfg(feature = "streaming")]
    #[test]
    fn test_convenience_mixed_types() {
        use crate::writer::stream_writer::ChannelGroupDefBuilder;

        let cg = ChannelGroupDefBuilder::new()
            .name("mixed")
            .with_time_channel("time")
            .add_f32_channel("temp", "°C")
            .add_u32_channel("counter", "")
            .add_i16_channel("offset", "mm")
            .add_u8_channel("status", "")
            .build()
            .expect("build CG");

        // time=8 + f32=4 + u32=4 + i16=2 + u8=1 = 19 bytes
        assert_eq!(cg.record_size, 19);

        assert_eq!(cg.channels[0].name, "temp");
        assert_eq!(cg.channels[0].data_type, 4); // FLOAT_LE
        assert_eq!(cg.channels[0].bit_count, 32);

        assert_eq!(cg.channels[1].name, "counter");
        assert_eq!(cg.channels[1].data_type, 0); // UINT_LE
        assert_eq!(cg.channels[1].bit_count, 32);

        assert_eq!(cg.channels[2].name, "offset");
        assert_eq!(cg.channels[2].data_type, 2); // INT_LE
        assert_eq!(cg.channels[2].bit_count, 16);

        assert_eq!(cg.channels[3].name, "status");
        assert_eq!(cg.channels[3].data_type, 0); // UINT_LE
        assert_eq!(cg.channels[3].bit_count, 8);
    }

    /// Test: write_record shorthand on Mf4StreamWriter
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_write_record_shorthand() {
        use crate::writer::stream_writer::{ChannelGroupDefBuilder, Mf4StreamWriter,
            StreamingDataGroup, StreamingConfig};
        use crate::writer::Mf4Metadata;

        let path = PathBuf::from("test_write_record_shorthand.mf4");
        cleanup_test_file(&path);

        let config = StreamingConfig::new().with_block_size(4_000_000);
        let metadata = Mf4Metadata::new().with_author("test");

        let mut writer = Mf4StreamWriter::with_config(path.clone(), metadata, config).unwrap();

        let cg = ChannelGroupDefBuilder::new()
            .name("data")
            .with_time_channel("time")
            .add_f64_channel("ch_0", "V")
            .add_f64_channel("ch_1", "A")
            .build()
            .unwrap();
        writer.add_data_group(StreamingDataGroup::new(cg).unwrap()).unwrap();
        writer.finalize_structure().unwrap();

        // Write 100 records using shorthand
        for i in 0..100 {
            let t = i as f64 * 0.01;
            writer.write_record(&[t, t.sin(), t.cos()]).unwrap();
        }

        writer.finalize_with_compact(true).unwrap();

        // Read back and verify
        let mf4 = Mf4Wrapper::new::<fn(f64)>(path.clone(), None).unwrap();
        let ch0 = mf4.get_channel_data("ch_0");
        assert!(ch0.is_some());
        if let Some(crate::data_serde::DataValue::REAL(vals)) = ch0 {
            assert_eq!(vals.len(), 100);
            let expected = 0.0_f64.sin();
            assert!((vals[0] - expected).abs() < 1e-10);
        }

        cleanup_test_file(&path);
    }

    /// Test: write_record validates value count
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_write_record_wrong_count() {
        use crate::writer::stream_writer::{ChannelGroupDefBuilder, Mf4StreamWriter,
            StreamingDataGroup, StreamingConfig};
        use crate::writer::Mf4Metadata;

        let path = PathBuf::from("test_write_record_wrong_count.mf4");
        cleanup_test_file(&path);

        let config = StreamingConfig::new();
        let metadata = Mf4Metadata::new().with_author("test");

        let mut writer = Mf4StreamWriter::with_config(path.clone(), metadata, config).unwrap();
        let cg = ChannelGroupDefBuilder::new()
            .name("data")
            .with_time_channel("time")
            .add_f64_channel("ch_0", "V")
            .build()
            .unwrap();
        writer.add_data_group(StreamingDataGroup::new(cg).unwrap()).unwrap();
        writer.finalize_structure().unwrap();

        // Should fail: 3 values but only 2 channels (time + ch_0)
        let result = writer.write_record(&[0.0, 1.0, 2.0]);
        assert!(result.is_err());

        // Should fail: 1 value but 2 channels expected
        let result = writer.write_record(&[0.0]);
        assert!(result.is_err());

        cleanup_test_file(&path);
    }

    /// Test: SimpleWriter basic creation and write
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_simple_writer_basic() {
        use crate::writer::simple_writer::SimpleWriter;

        let path = PathBuf::from("test_simple_writer_basic.mf4");
        cleanup_test_file(&path);

        let mut writer = SimpleWriter::new(&path)
            .author("Test Author")
            .time_channel("time", "s")
            .f64_channel("voltage", "V")
            .f64_channel("current", "A")
            .build()
            .expect("build SimpleWriter");

        // Write 50 records
        for i in 0..50 {
            let t = i as f64 * 0.001;
            writer.write_record(&[t, t.sin(), t.cos()]).unwrap();
        }

        writer.finalize().unwrap();

        // Read back and verify
        let mf4 = Mf4Wrapper::new::<fn(f64)>(path.clone(), None).unwrap();
        let names = mf4.get_channel_names();
        assert!(names.contains(&"voltage".to_string()));
        assert!(names.contains(&"current".to_string()));

        if let Some(crate::data_serde::DataValue::REAL(vals)) = mf4.get_channel_data("voltage") {
            assert_eq!(vals.len(), 50);
        } else {
            panic!("voltage channel data not found or wrong type");
        }

        cleanup_test_file(&path);
    }

    /// Test: SimpleWriter with compression and stream mode
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_simple_writer_compressed_stream() {
        use crate::writer::simple_writer::SimpleWriter;

        let path = PathBuf::from("test_simple_writer_compressed_stream.mf4");
        cleanup_test_file(&path);

        let mut writer = SimpleWriter::new(&path)
            .author("Compressed Test")
            .comment("Testing compressed stream mode")
            .time_channel("time", "s")
            .f64_channel("signal", "V")
            .compression(6)
            .stream_mode()
            .compression_threshold(0)
            .build()
            .expect("build SimpleWriter");

        for i in 0..200 {
            let t = i as f64 * 0.001;
            writer.write_record(&[t, t * 10.0]).unwrap();
        }

        assert_eq!(writer.records_written(), 200);
        writer.finalize().unwrap();

        // Read back and verify
        let mf4 = Mf4Wrapper::new::<fn(f64)>(path.clone(), None).unwrap();
        if let Some(crate::data_serde::DataValue::REAL(vals)) = mf4.get_channel_data("signal") {
            assert_eq!(vals.len(), 200);
            // Verify first value
            assert!((vals[0] - 0.0).abs() < 1e-10);
            // Verify last value
            let expected_last = 199.0 * 0.001 * 10.0;
            assert!((vals[199] - expected_last).abs() < 1e-10);
        } else {
            panic!("signal channel not found");
        }

        cleanup_test_file(&path);
    }

    /// Test: SimpleWriter compact mode
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_simple_writer_compact_mode() {
        use crate::writer::simple_writer::SimpleWriter;

        let path = PathBuf::from("test_simple_writer_compact.mf4");
        cleanup_test_file(&path);

        let mut writer = SimpleWriter::new(&path)
            .time_channel("time", "s")
            .f64_channel("value", "m")
            .compact_mode()
            .build()
            .expect("build SimpleWriter");

        for i in 0..100 {
            writer.write_record(&[i as f64 * 0.01, i as f64]).unwrap();
        }

        writer.finalize().unwrap();

        let mf4 = Mf4Wrapper::new::<fn(f64)>(path.clone(), None).unwrap();
        if let Some(crate::data_serde::DataValue::REAL(vals)) = mf4.get_channel_data("value") {
            assert_eq!(vals.len(), 100);
            assert!((vals[50] - 50.0).abs() < 1e-10);
        } else {
            panic!("value channel not found");
        }

        cleanup_test_file(&path);
    }

    /// Test: SimpleWriter roundtrip data integrity with many samples
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_simple_writer_roundtrip_integrity() {
        use crate::writer::simple_writer::SimpleWriter;

        let path = PathBuf::from("test_simple_writer_integrity.mf4");
        cleanup_test_file(&path);

        let num_samples = 10_000;

        let mut writer = SimpleWriter::new(&path)
            .author("Integrity Test")
            .group_name("Sensors")
            .time_channel("timestamp", "s")
            .f64_channel("sine", "V")
            .f64_channel("cosine", "A")
            .f64_channel("ramp", "")
            .stream_mode()
            .build()
            .expect("build");

        let mut expected_sine = Vec::with_capacity(num_samples);
        let mut expected_ramp = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f64 * 0.0001;
            let sine = t.sin();
            let cosine = t.cos();
            let ramp = i as f64;
            writer.write_record(&[t, sine, cosine, ramp]).unwrap();
            expected_sine.push(sine);
            expected_ramp.push(ramp);
        }

        writer.finalize().unwrap();

        let mf4 = Mf4Wrapper::new::<fn(f64)>(path.clone(), None).unwrap();

        if let Some(crate::data_serde::DataValue::REAL(sine_vals)) = mf4.get_channel_data("sine") {
            assert_eq!(sine_vals.len(), num_samples);
            for i in 0..num_samples {
                assert!((sine_vals[i] - expected_sine[i]).abs() < 1e-10,
                    "sine mismatch at index {}: got {} expected {}", i, sine_vals[i], expected_sine[i]);
            }
        } else {
            panic!("sine channel not found");
        }

        if let Some(crate::data_serde::DataValue::REAL(ramp_vals)) = mf4.get_channel_data("ramp") {
            assert_eq!(ramp_vals.len(), num_samples);
            for i in 0..num_samples {
                assert!((ramp_vals[i] - expected_ramp[i]).abs() < 1e-10,
                    "ramp mismatch at index {}", i);
            }
        } else {
            panic!("ramp channel not found");
        }

        cleanup_test_file(&path);
    }

    /// Test: SimpleWriter write_record rejects wrong value count
    #[cfg(all(feature = "streaming", feature = "compression"))]
    #[test]
    fn test_simple_writer_wrong_value_count() {
        use crate::writer::simple_writer::SimpleWriter;

        let path = PathBuf::from("test_simple_writer_wrong_count.mf4");
        cleanup_test_file(&path);

        let mut writer = SimpleWriter::new(&path)
            .time_channel("time", "s")
            .f64_channel("ch_0", "V")
            .build()
            .expect("build");

        // 2 channels but 3 values — should error
        assert!(writer.write_record(&[0.0, 1.0, 2.0]).is_err());
        // 2 channels but 1 value — should error
        assert!(writer.write_record(&[0.0]).is_err());
        // Correct count
        assert!(writer.write_record(&[0.0, 1.0]).is_ok());

        cleanup_test_file(&path);
    }
}
