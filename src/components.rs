pub mod cn;
pub mod cc;
pub mod cg;
pub mod si;

pub mod components_test {
    use crate::components::cg::channelgroup::*;
    use crate::components::cn::channel::Channel;
    use rust_embed::RustEmbed;
    use std::io::BufReader;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Mutex;
    use rstest::*;

    #[derive(RustEmbed)]
    #[folder = "test/"]
    #[prefix = "test/"]
    struct Asset;

    #[fixture]
    #[once]
    fn buffer() -> Mutex<BufReader<File>> {
        let file_data = Asset::get("test/1.mf4").unwrap();
        let mut new_file = File::create("temp.mf4").unwrap();
        new_file.write(file_data.data.as_ref()).unwrap();
        let file = File::open("temp.mf4").unwrap();
        let buf= BufReader::new(file);
        Mutex::new(buf)
    }

    #[rstest]
    fn test_new(buffer: &Mutex<BufReader<File>>) {
        let offset = 0x6400;
        let mut buf = buffer.lock().unwrap();
        let cg: ChannelGroup = ChannelGroup::new(&mut buf, offset).unwrap();
        println!("{:?}", cg);
    }

    #[rstest]
    #[case(0x6250)]
    #[case(0x6328)]
    #[case(0x64A0)]
    #[case(0x6578)]
    #[case(0x6650)]
    fn test_channel_new_0(buffer: &Mutex<BufReader<File>>, #[case] offset: u64) {
        let mut buf = buffer.lock().unwrap();
        println!("offset = 0x{:X}", offset);
        let channel: Channel = Channel::new(&mut buf, offset).unwrap();
        println!("{:?}", channel);
    }
}