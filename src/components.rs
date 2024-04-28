/*
For test purposes
*/

pub mod cn;
pub mod cc;
pub mod cg;
pub mod si;
pub mod dg;
pub mod dx;
pub mod ca;


#[cfg(test)]
pub mod components_test {
    use crate::components::cg::channelgroup::*;
    use crate::components::cn::channel::Channel;
    use crate::components::cc::conversion::*;
    use crate::components::dg::datagroup::DataGroup;
    use crate::components::ca::channelarray::ChannelArray;
    use super::dx::dataxxx::{DataLink, VirtualBuf}; 
    use crate::data_serde::DataValue;
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
    fn test_new_cg(buffer: &Mutex<BufReader<File>>) {
        let offset = 0x6400;
        let mut buf = buffer.lock().unwrap();
        let cg: ChannelGroup = ChannelGroup::new(&mut buf, offset).unwrap();
        println!("{:?}", cg);
    }

    #[rstest]
    fn test_cg_get_channel_name(buffer: &Mutex<BufReader<File>>) {
        let offset = 0x6400;
        let mut buf = buffer.lock().unwrap();
        let cg: ChannelGroup = ChannelGroup::new(&mut buf, offset).unwrap();
        println!("{:?}", cg.get_channel_names());
    }

    #[rstest]
    fn test_dg_new(buffer: &Mutex<BufReader<File>>) {
        let offset: u64 = 0x8CB0;
        let mut buf = buffer.lock().unwrap();
        let dg: DataGroup = DataGroup::new(&mut buf, offset).unwrap();
        println!("{}", dg);
        println!("is sorted: {}", dg.is_sorted());
        let map = dg.create_map();
        println!("{:?}", map.keys().collect::<Vec<&String>>());
        let var = map.get("$CalibrationLog").unwrap();
        println!("{:?}", var.get_channel());
        let value = var.yield_channel_data(&mut buf).unwrap();
        println!("{:?}", value);
    }

    #[rstest]
    fn test_dg_new_1(buffer: &Mutex<BufReader<File>>) {
        let offset: u64 = 0x8cf0;
        let mut buf = buffer.lock().unwrap();
        let dg: DataGroup = DataGroup::new(&mut buf, offset).unwrap();
        println!("{}", dg);
        println!("is sorted: {}", dg.is_sorted());
        let channel_map = dg.create_map();
        println!("{:?}", channel_map.keys().collect::<Vec<&String>>());
        let cl = channel_map.get("ASAM.M.SCALAR.UBYTE.RAT_FUNC.IDENT.STATUS_STRING").unwrap();
        println!("{}", cl.get_channel());
        let value = cl.yield_channel_data(&mut buf).unwrap();
        //println!("{:?}", value);
        let raw = cl.get_channel().get_data_raw(&mut buf, cl.get_data_group(), cl.get_channel_group()).unwrap();
        assert_eq!(value, raw);
        let value_conv = if let DataValue::UINT8(val) = value {
            val.iter().map(|x| cl.get_channel().get_conversion().transform_value(*x)).collect::<Vec<f64>>()
        } else {
            vec![]
        };
        println!("{:?}", value_conv);
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

    #[rstest]
    fn test_cc1(buffer: &Mutex<BufReader<File>>) {
        let offset = 0x52E8;
        let mut buf = buffer.lock().unwrap();
        let cc = Conversion::new(&mut buf, offset).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.transform_value::<f64, f64>(1000.0), 2000.0);   // linear with p2 = 2
    }

    #[rstest]
    fn test_cc2(buffer: &Mutex<BufReader<File>>) {
        let offset = 0x5348;
        let mut buf = buffer.lock().unwrap();
        let cc = Conversion::new(&mut buf, offset).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.transform_value::<f64, f64>(1000.0), 1000.0);   // linear with p2 = 1
    }

    #[rstest]
    fn test_cc3(buffer: &Mutex<BufReader<File>>) {
        let offset = 0x53A8;
        let mut buf = buffer.lock().unwrap();
        let cc = Conversion::new(&mut buf, offset).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "hundredfive");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.convert_to_text::<f64>(&mut buf, 0.5).unwrap(), "Zero_to_one".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 1u8).unwrap(), "Zero_to_one".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 2.5).unwrap(), "two_to_three".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 105).unwrap(), "hundredfive".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 105.1).unwrap(), "".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 15.1).unwrap(), "fourteen_to_seventeen".to_string());
    }

    #[rstest]
    fn test_cc4(buffer: &Mutex<BufReader<File>>) {
        let offset = 0x5508;
        let mut buf = buffer.lock().unwrap();
        let cc = Conversion::new(&mut buf, offset).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "unknown signal type");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.convert_to_text(&mut buf, 15.1).unwrap(), "unknown signal type".to_string());   // linear with p2 = 1
        assert_eq!(cc.convert_to_text(&mut buf, 3).unwrap(), "Sinus".to_string());   // linear with p2 = 1
        assert_eq!(cc.convert_to_text(&mut buf, 2).unwrap(), "Square".to_string());   // linear with p2 = 1
        assert_eq!(cc.convert_to_text(&mut buf, 1).unwrap(), "SawTooth".to_string());   // linear with p2 = 1
    }

    #[rstest]
    fn test_dl_new_0(buffer: &Mutex<BufReader<File>>) {
        let offset: u64 = 0xdbc0;
        let mut buf = buffer.lock().unwrap();
        let dl: DataLink = DataLink::new(&mut buf, offset).unwrap();
        println!("{:?}", dl);
    }

    #[rstest]
    fn test_dl_new_1(buffer: &Mutex<BufReader<File>>) {
        let offset: u64 = 0x8F10;
        let mut buf = buffer.lock().unwrap();
        let dl: DataLink = DataLink::new(&mut buf, offset).unwrap();
        println!("{:?}", dl);
        assert_eq!(dl.get_num_of_blocks(), 3);
        assert_eq!(dl.get_start_offsets_in_file(), &vec![59528, 321912, 597648]);
        let mut data_buf = [0u8; 20];
        dl.read_virtual_buf(&mut buf, 262_134, &mut data_buf).unwrap(); // this read will span across two blocks
        assert_eq!(data_buf, [13, 1, 0, 0, 23, 0, 213, 57, 2, 0, 64, 39, 68, 1, 3, 101, 201, 69, 169, 1]);
    }

    #[rstest]
    fn test_dl_new_2(buffer: &Mutex<BufReader<File>>) {
        let offset: u64 = 0x9BD8;  // this dlblock points to one DT: 0x9D328
        let mut buf = buffer.lock().unwrap();
        let dl: DataLink = DataLink::new(&mut buf, offset).unwrap();
        println!("{:?}", dl);
        let mut data_buf = [0u8; 10];
        dl.read_virtual_buf(&mut buf, 0, &mut data_buf).unwrap();
        assert_eq!(dl.get_num_of_blocks(), 1);
        assert_eq!(dl.get_total_len(), 173628);
        assert_eq!(data_buf, [32, 72, 146, 234, 150, 120, 139, 63, 244, 25]);
        dl.read_virtual_buf(&mut buf, 173618, &mut data_buf).unwrap();
        assert_eq!(data_buf, [48, 212, 40, 64, 231, 153, 171, 119, 114, 0]);
    }

    #[rstest]
    fn test_new_ca() {
        let mut buf = BufReader::new(File::open("./test/demo.mf4").unwrap());
        let ca = ChannelArray::new(&mut buf, 0xD8E0).unwrap();
        println!("{:?}", ca);
    }
}