use mf4_parse::parser::Mf4Wrapper;
use mf4_parse::components::dg::datagroup::ChannelLink;
use std::path::PathBuf;
use std::time::{Instant, Duration};



fn display_channel_info(channel_name: &str, mf4: &Mf4Wrapper) {
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

use mf4_parse::components::cg::channelgroup::CN_TIME;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let mut new: Mf4Wrapper = Mf4Wrapper::new(PathBuf::from("test/Vector_Unsorted_VLSD.MF4"))?;
    let duration: Duration = Instant::now() - start_time;
    println!("load mf4 file time: {:?}", duration.as_secs_f64());
    unsafe {
        println!("CN time {:?}", CN_TIME);
    }
    println!("channel names: {:?}", new.get_channel_names());
    display_channel_info("Comment", &new);
    display_channel_info("Write", &new);
    println!("channel data: {:?}", new.get_channel_data("WriteTextToWriteWindow"));
    println!("Corresponding master channel data: {:?}", new.get_channel_master_data("WriteTextToWriteWindow"));
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
        let mf4: Mf4Wrapper = Mf4Wrapper::new(PathBuf::from("test/demo.mf4")).unwrap();
        let channel_names: Vec<String> = mf4.get_channel_names();
        println!("{:?}", channel_names);
        display_channel_info("Nested_structures", &mf4);
        display_channel_info("Channel_lookup_with_default_axis", &mf4);
        let new: Mf4Wrapper = Mf4Wrapper::new(PathBuf::from("test/string_and_array.mf4")).unwrap();
        display_channel_info("Channel_lookup_with_default_axis[0][0][2]", &new);
        let d: mf4_parse::data_serde::DataValue = new.get_channel_data("Channel_lookup_with_default_axis[0][0][2]").unwrap();
        println!("{:?}\n value ends\n", d);
    }
}