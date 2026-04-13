use mf4_parse::writer::stream_writer::{
    ChannelGroupDefBuilder, Mf4StreamWriter, StreamingConfig, StreamingDataGroup,
};
use mf4_parse::writer::simple_writer::SimpleWriter;
use mf4_parse::writer::Mf4Metadata;
use std::path::PathBuf;
use std::time::Instant;

const NUM_CHANNELS: usize = 3;

fn bench_write(label: &str, num_samples: usize, config: StreamingConfig, compact: bool) {
    let path = PathBuf::from(format!("temp_bench_{}.mf4", label.replace(' ', "_")));

    let mut cg_builder = ChannelGroupDefBuilder::new()
        .name("bench_group")
        .with_time_channel("time");

    for ch_idx in 0..NUM_CHANNELS {
        cg_builder = cg_builder.add_f64_channel(&format!("ch_{}", ch_idx), "V");
    }

    let cg = cg_builder.build().expect("build CG");
    let metadata = Mf4Metadata::new()
        .with_author("bench")
        .with_comment("perf test");

    let start = Instant::now();

    let mut writer = Mf4StreamWriter::with_config(path.clone(), metadata, config.clone()).unwrap();
    writer
        .add_data_group(StreamingDataGroup::new(cg).unwrap())
        .unwrap();
    writer.finalize_structure().unwrap();

    for i in 0..num_samples {
        let t = i as f64 * 0.001;
        writer.start_record(0, 0).unwrap();
        writer.set_channel_value("time", t).unwrap();
        writer.set_channel_value("ch_0", t.sin()).unwrap();
        writer.set_channel_value("ch_1", t.cos()).unwrap();
        writer.set_channel_value("ch_2", t * 2.0).unwrap();
        writer.flush_record().unwrap();
    }

    writer.finalize_with_compact(compact).unwrap();
    let elapsed = start.elapsed();

    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let records_per_sec = num_samples as f64 / elapsed.as_secs_f64();

    println!(
        "{:<40} | {:>10} samples | {:>8.2}s | {:>12.0} rec/s | {:>8.1} MB",
        label,
        num_samples,
        elapsed.as_secs_f64(),
        records_per_sec,
        file_size as f64 / (1024.0 * 1024.0),
    );

    let _ = std::fs::remove_file(&path);
}

/// Benchmark using SimpleWriter API
fn bench_simple(label: &str, num_samples: usize, compression: u8, compact: bool) {
    let path = PathBuf::from(format!("temp_bench_{}.mf4", label.replace(' ', "_")));

    let start = Instant::now();

    let mut builder = SimpleWriter::new(&path)
        .author("bench")
        .time_channel("time", "s")
        .f64_channel("ch_0", "V")
        .f64_channel("ch_1", "A")
        .f64_channel("ch_2", "W");

    if compression > 0 {
        builder = builder.compression(compression);
    }
    if compact {
        builder = builder.compact_mode();
    } else {
        builder = builder.stream_mode();
    }

    let mut writer = builder.build().unwrap();

    for i in 0..num_samples {
        let t = i as f64 * 0.001;
        writer.write_record(&[t, t.sin(), t.cos(), t * 2.0]).unwrap();
    }

    writer.finalize().unwrap();
    let elapsed = start.elapsed();

    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let records_per_sec = num_samples as f64 / elapsed.as_secs_f64();

    println!(
        "{:<40} | {:>10} samples | {:>8.2}s | {:>12.0} rec/s | {:>8.1} MB",
        label,
        num_samples,
        elapsed.as_secs_f64(),
        records_per_sec,
        file_size as f64 / (1024.0 * 1024.0),
    );

    let _ = std::fs::remove_file(&path);
}

fn main() {
    println!("MF4 Stream Writer Performance Benchmark");
    println!("{}", "=".repeat(105));
    println!(
        "{:<40} | {:>10}         | {:>8}  | {:>14}  | {:>8}",
        "Configuration", "Samples", "Time", "Throughput", "Size"
    );
    println!("{}", "-".repeat(105));

    // Full API benchmarks
    // block_size controls DT (uncompressed) buffer flushing; DZ blocks are always ≤4MB internally
    let config_plain = StreamingConfig::new().with_block_size(4_000_000);
    let config_compressed = StreamingConfig::new()
        .with_block_size(4_000_000)
        .with_compression_level(6);

    bench_write("full API: compact, no compression", 1_000_000, config_plain.clone(), true);
    bench_write("full API: compact, compressed", 1_000_000, config_compressed.clone(), true);
    bench_write("full API: stream, no compression", 1_000_000, config_plain.clone(), false);
    bench_write("full API: stream, compressed", 1_000_000, config_compressed.clone(), false);

    println!("{}", "-".repeat(105));

    // SimpleWriter benchmarks (same configs)
    bench_simple("SimpleWriter: compact, no compression", 1_000_000, 0, true);
    bench_simple("SimpleWriter: compact, compressed", 1_000_000, 6, true);
    bench_simple("SimpleWriter: stream, no compression", 1_000_000, 0, false);
    bench_simple("SimpleWriter: stream, compressed", 1_000_000, 6, false);

    // 5M samples comparison
    println!("{}", "-".repeat(105));
    bench_write("full API: stream 5M", 5_000_000, config_plain.clone(), false);
    bench_simple("SimpleWriter: stream 5M", 5_000_000, 0, false);
    bench_write("full API: stream 5M compressed", 5_000_000, config_compressed.clone(), false);
    bench_simple("SimpleWriter: stream 5M compressed", 5_000_000, 6, false);

    println!("{}", "=".repeat(105));
}
