//! MF4 Parse CLI
//!
//! A command-line tool for reading and processing MF4 files.
//!
//! Usage:
//!   mf4_parse_cli                    - Read and display info from test/demo.mf4
//!   mf4_parse_cli <file.mf4>         - Read and display info from specified file
//!   mf4_parse_cli --sort <in> <out>  - Sort an MF4 file (requires "write" feature)
//!
//! For examples on generating MF4 files, see the examples/ directory:
//!   - time_series_demo.rs          - Multi-rate time series (single DG)
//!   - time_series_one_dg_per_cg.rs - Multi-rate time series (separate DGs)
//!   - streaming_demo.rs            - Real-time streaming write
//!   - compression_demo.rs          - Compressed vs uncompressed files

use mf4_parse::ChannelLink;
use mf4_parse::Mf4Wrapper;
use std::env;
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
    let args: Vec<String> = env::args().collect();

    // Handle --sort command: mf4_parse_cli --sort <input> <output>
    #[cfg(feature = "write")]
    if args.len() >= 2 && args[1] == "--sort" {
        if args.len() < 4 {
            eprintln!("Usage: mf4_parse_cli --sort <input.mf4> <output.mf4>");
            std::process::exit(1);
        }
        let input = PathBuf::from(&args[2]);
        let output = PathBuf::from(&args[3]);
        println!("Sorting MF4: {} -> {}", input.display(), output.display());
        mf4_parse::sort::sort_mf4(input, output)?;
        println!("Sort complete.");
        return Ok(());
    }

    // Default demo mode
    let file_path = if args.len() >= 2 && args[1] != "--sort" {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("test/demo.mf4")
    };

    let mf4: Mf4Wrapper = Mf4Wrapper::new::<fn(f64)>(file_path.clone(), None)?;

    println!("Reading MF4 file: {}", file_path.display());
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

    Ok(())
}

#[cfg(test)]
pub mod test {
    use mf4_parse::parser::Mf4Wrapper;
    use rstest::*;
    use std::path::PathBuf;

    use super::display_channel_info;

    #[rstest]
    fn mf4_wrapper_test() {
        let mf4: Mf4Wrapper =
            Mf4Wrapper::new::<fn(f64)>(PathBuf::from("test/demo.mf4"), None).unwrap();
        let channel_names: Vec<String> = mf4.get_channel_names();
        println!("{:?}", channel_names);
        display_channel_info("Nested_structures", &mf4);
        display_channel_info("Channel_lookup_with_default_axis", &mf4);
        let new: Mf4Wrapper =
            Mf4Wrapper::new::<fn(f64)>(PathBuf::from("test/string_and_array.mf4"), None).unwrap();
        display_channel_info("Channel_lookup_with_default_axis[0][0][2]", &new);
        let d: mf4_parse::data_serde::DataValue =
            new.get_channel_data("Channel_lookup_with_default_axis[0][0][2]")
                .unwrap();
        println!("{:?}\n value ends\n", d);
    }
}
