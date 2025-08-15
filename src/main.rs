use mf4_parse::components::cn::channel;
use mf4_parse::Mf4Wrapper;
use mf4_parse::ChannelLink;
use std::path::PathBuf;
use std::time::Instant;




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
    let mf4: Mf4Wrapper = Mf4Wrapper::new(PathBuf::from(r"D:\ETASData\INCA7.2\Measure\jingtai 17-07-2025 03_40_33 PM.mf4"), Some(&|_f|{})).unwrap();
    let start = Instant::now();
    let channel_name = ["IVE_BdyVZRear"];   // CDCHndlLH_TurnStRght IVE_BdyVZRear CDCInput_UBat  
    for channel_name in channel_name {
        let channel_data: Vec<f64> = mf4.get_channel_data(channel_name).unwrap().try_into()?;
        println!("data length: {:?}", channel_data.len());
        println!("1 Time elapsed: {:?} channel data   first 10 samples {:?}", start.elapsed(), &channel_data[0..10]);
        let master: Vec<f64> = mf4.get_channel_data(channel_name).unwrap().try_into()?;
        println!("master length: {:?}", master.len());
        println!("2 Time elapsed: {:?} master data", start.elapsed());
    }
    Ok(())
}


#[cfg(test)]
pub mod test {
    use rstest::*;
    use mf4_parse::parser::Mf4Wrapper;
    use std::path::PathBuf;
    use super::display_channel_info;
    use std::time::Instant;

    #[rstest]
    fn mf4_wrapper_test() {
        let mf4: Mf4Wrapper = Mf4Wrapper::new::<fn(f64)>(PathBuf::from("test/demo.mf4"), None).unwrap();
        let channel_names: Vec<String> = mf4.get_channel_names();
        println!("{:?}", channel_names);
        display_channel_info("Nested_structures", &mf4);
        display_channel_info("Channel_lookup_with_default_axis", &mf4);
        let new: Mf4Wrapper = Mf4Wrapper::new::<fn(f64)>(PathBuf::from("test/string_and_array.mf4"), None).unwrap();
        display_channel_info("Channel_lookup_with_default_axis[0][0][2]", &new);
        let d: mf4_parse::data_serde::DataValue = new.get_channel_data("Channel_lookup_with_default_axis[0][0][2]").unwrap();
        println!("{:?}\n value ends\n", d);
    }

    #[rstest]
    fn mf4_wrapper_test2() {
        let mf4: Mf4Wrapper = Mf4Wrapper::new::<fn(f64)>(PathBuf::from(r"D:\ETASData\INCA7.2\Measure\jingtai 17-07-2025 03_40_33 PM.mf4"), None).unwrap();
        let start = Instant::now();
        let channel_name = "IVE_BdyVZRear";
        let _ = mf4.get_channel_data(channel_name).unwrap();
        //println!("{:?}", channel_data);
        println!("1 Time elapsed: {:?} channel data", start.elapsed());
        let _ = mf4.get_channel_master_data(channel_name).unwrap();
        //println!("{:?}", master);
        println!("2 Time elapsed: {:?} master data", start.elapsed());
    }
}