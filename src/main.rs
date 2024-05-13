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


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let new: Mf4Wrapper = Mf4Wrapper::new(PathBuf::from("test/12.mf4"))?;
    let duration: Duration = Instant::now() - start_time;
    println!("load mf4 file time: {:?}", duration.as_secs_f64());
    //println!("{:?}", new.get_channel_names());
    display_channel_info("CAN_DataFrame_60.CAN_DataFrame.CAN10", &new);
    if let Some(chs) = new.check_duplicated() {
        println!("duplicated channels: {:?}", chs);
    }
    let data = new.get_channel_data("CAN_DataFrame_60.CAN_DataFrame.CAN10").unwrap();
    match &data {
        mf4_parse::data_serde::DataValue::STRUCT(value) => {
            println!("{:?}", value.get("CAN_DataFrame.BRS").unwrap());
        },
        _ => {
            println!("not struct data");
        }
    }
    //println!("{:?}", data);

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
        let d: mf4_parse::data_serde::DataValue = mf4.get_channel_data("Nested_structures").unwrap();
        println!("{:?}\n value ends\n", d);
        display_channel_info("Nested_structures", &mf4);
        display_channel_info("Channel_lookup_with_default_axis", &mf4);
        let new: Mf4Wrapper = Mf4Wrapper::new(PathBuf::from("test/string_and_array.mf4")).unwrap();
        display_channel_info("Channel_lookup_with_default_axis[0][0][2]", &new);
        let d: mf4_parse::data_serde::DataValue = new.get_channel_data("Channel_lookup_with_default_axis[0][0][2]").unwrap();
        println!("{:?}\n value ends\n", d);
    }
}