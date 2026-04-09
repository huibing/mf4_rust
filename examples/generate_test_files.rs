/// Generate test MF4 files for manual verification with MDA/other tools.
///
/// Usage: cargo run --example generate_test_files --features "streaming,compression"

use mf4_parse::writer::stream_writer::*;
use std::path::PathBuf;

fn main() {
    let output_dir = PathBuf::from("temp_test_output");

    // 1. DL→DT chain (uncompressed, small blocks)
    println!("Generating: dl_chain_uncompressed.mf4");
    generate_file(
        output_dir.join("dl_chain_uncompressed.mf4"),
        StreamingConfig::new().with_block_size(100),
        500,
        false,
    );

    // 2. HL→DL→DZ chain (compressed, small blocks)
    println!("Generating: hl_dl_dz_chain_compressed.mf4");
    generate_file(
        output_dir.join("hl_dl_dz_chain_compressed.mf4"),
        StreamingConfig::new()
            .with_block_size(200)
            .with_compression()
            .with_compression_threshold(0),
        500,
        false,
    );

    // 3. Compact single DT (no DL chain)
    println!("Generating: compact_single_dt.mf4");
    generate_file(
        output_dir.join("compact_single_dt.mf4"),
        StreamingConfig::new().with_block_size(100),
        500,
        true,
    );

    // 4. Compact single DZ (compressed)
    println!("Generating: compact_single_dz.mf4");
    generate_file(
        output_dir.join("compact_single_dz.mf4"),
        StreamingConfig::new()
            .with_block_size(100)
            .with_compression()
            .with_compression_threshold(0),
        500,
        true,
    );

    // 5. Larger file with DL chain (uncompressed, default block size)
    println!("Generating: large_dl_chain.mf4");
    generate_file(
        output_dir.join("large_dl_chain.mf4"),
        StreamingConfig::new().with_block_size(4096),
        10000,
        false,
    );

    // 6. Larger file with HL→DL→DZ chain (compressed, default block size)
    println!("Generating: large_hl_dl_dz_chain.mf4");
    generate_file(
        output_dir.join("large_hl_dl_dz_chain.mf4"),
        StreamingConfig::new()
            .with_block_size(4096)
            .with_compression()
            .with_compression_threshold(0),
        10000,
        false,
    );

    println!("\nAll files generated in: {}", output_dir.display());
}

fn generate_file(path: PathBuf, config: StreamingConfig, num_records: usize, compact: bool) {
    use mf4_parse::writer::{ChannelDef, ChannelGroupDef, Mf4Metadata};

    let metadata = Mf4Metadata::default();
    let mut writer = Mf4StreamWriter::with_config(
        path.clone(),
        metadata,
        config,
    ).unwrap();

    let time_def = ChannelDef::new_master("time");
    let signal_def = ChannelDef::new("Signal")
        .data_type(4)  // FLOAT64
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
    println!("  -> {} ({} bytes)", path.display(), std::fs::metadata(&path).unwrap().len());
}
