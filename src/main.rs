use mf4_parse::parser::Mf4Wrapper;
use mf4_parse::components::dg::datagroup::ChannelLink;
use std::path::PathBuf;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mf4 = Mf4Wrapper::new(PathBuf::from("test/demo.mf4"))?;
    let channel_names = mf4.get_channel_names();
    println!("{:?}", channel_names);
    let _ = mf4.get_channel_data("ZONE_2D_CRC\0\0\0\0");
    /* println!("{:?}\n value ends\n", value);
    if let Some(ChannelLink(cn, cg, _)) = mf4.get_channel_link("ZONE_2D_CRC\0\0\0\0") {
        println!("channel comment: {:?}", cn.get_comment());
        println!("channel group comment: {:?}", cg.get_comment());
        println!("channel group source: {:?}", cg.get_acq_name());
        println!("channel group source info: {}", cg.get_acq_source());
    } */
    Ok(())
}