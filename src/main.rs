use mf4_parse::parser::Mf4Wrapper;
use mf4_parse::components::dg::datagroup::ChannelLink;
use std::path::PathBuf;

fn display_channel_info(channel_name: &str, mf4: &Mf4Wrapper) {
    if let Some(ChannelLink(cn, cg, _)) = mf4.get_channel_link(channel_name) {
        println!("channel info: \n{}", cn);
        println!("channel group comment: {:?}", cg.get_comment());
        println!("channel group source: {:?}", cg.get_acq_name());
        println!("channel group source info: {}", cg.get_acq_source());
    } else {
        println!("no channel info found for {}", channel_name);
    }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mf4 = Mf4Wrapper::new(PathBuf::from("test/demo.mf4"))?;
    let channel_names = mf4.get_channel_names();
    println!("{:?}", channel_names);
    let d = mf4.get_channel_data("Channel_value_range_to_text").unwrap();
    println!("{:?}\n value ends\n", d);
    display_channel_info("Channel_bytearay", &mf4);
    Ok(())
}