pub mod parser {
    use mf4_parse::block::*;
    use rust_embed::RustEmbed;
    use std::io::{BufReader, Seek, Read, SeekFrom};
    use std::path::PathBuf;
    use std::fs::File;
    use std::str::FromStr;
    use byteorder::{LittleEndian, ByteOrder};
    use std::collections::HashMap;
    use chrono::DateTime;


    #[derive(RustEmbed)]
    #[folder = "config/"]
    #[prefix = "config/"]
    struct Asset;   // compile config file asset to binary


    pub struct MdfInfo {  //  information that stored in mdf ID and HD block
        pub version: String,
        pub version_num: u16,
        pub time_stamp: u64,
        pub date_time: String,
        pub first_dg_offset: u64,
    }

    impl MdfInfo {
        pub fn new() -> Self{
            MdfInfo {
                version: "4.10".to_string(),
                version_num: 410,
                time_stamp: 0u64,
                date_time: "".to_string(),
                first_dg_offset: 0u64,
            }
        }
    }

    lazy_static! {
        static ref DESC_MAP: HashMap<String, BlockDesc> = {
            let mut m = HashMap::new();
            let block_types = ["DG", "HD", "CG", "TX", "MD", "CN"];
            block_types.into_iter().for_each(|key| {
                let desc = parse_toml(key.to_lowercase().as_str()).unwrap();
                m.insert(key.to_string(), desc);
            });
            m
        };
    }

    pub fn get_block_desc<'a>(file: &'a mut BufReader<File>, offset: u64) -> Result<&'static BlockDesc, Box<dyn std::error::Error>>{
        //use file offset to acquire the actual block type and its block desc
        if offset == 0 {
            return Err("Invalid offset".into());
        }
        let mut buf: [u8; 4] = [0u8;4];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buf)?;
        let block_type: String = String::from_utf8(buf[2..].to_vec())?;
        Ok(DESC_MAP.get(&block_type).unwrap())
    }

    pub fn parse_toml(block_name: &str) -> Result<BlockDesc, Box<dyn std::error::Error>> {
        let mut path: PathBuf = PathBuf::from("config/");
        path.push(block_name);
        path.set_extension("toml");
        let toml_file: rust_embed::EmbeddedFile = Asset::get(path.to_str().ok_or("")?).ok_or("")?;
        Ok(toml::from_str(std::str::from_utf8(toml_file.data.as_ref())?)?)
    }

    pub fn parse_mdf_header(file: &mut BufReader<File>, mdf: &mut MdfInfo) -> Result<(), Box<dyn std::error::Error>>{
        // manually parse id block
        file.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8;8];
        let mut two_bytes: [u8;2] = [0u8;2];
        file.read_exact(&mut buf)?;
        if std::str::from_utf8(&buf).unwrap() != "MDF     " {
            return Err("not a mdf file".into());
        }
        // read version
        file.read_exact(&mut buf)?;
        let version = String::from_utf8(buf.to_vec()).unwrap();
        mdf.version = version.trim().to_string();

        file.seek(SeekFrom::Current(12))?; // skip 12 bytes
        // read version number
        file.read_exact(&mut two_bytes)?;
        mdf.version_num = LittleEndian::read_u16(&two_bytes);
        if mdf.version_num <= 400 {
            panic!("unsupported version: {}", mdf.version_num);   // do not support any version below 4.00
        }
        file.seek(SeekFrom::Current(30))?; // skip 30 bytes
        file.read_exact(&mut two_bytes)?; //id_unfin_flags
        file.read_exact(&mut two_bytes)?; //id_custom_unfin_flags
        let offset = file.stream_position().unwrap();
        //parse header HD block
        let block: &BlockDesc = get_block_desc(file, 0x40)?;
        let header_info: BlockInfo = block.try_parse_buf(file, offset)?;
        let fisrt_dg_offset: u64 = header_info.get_link_offset("hd_dg_first").unwrap();
        //parse time stamp
        let time_stamp_v = header_info.get_data_value("hd_start_time_ns").unwrap();
        let t: Vec<u64> = time_stamp_v.clone().try_into().unwrap();
        let dt = DateTime::from_timestamp_nanos(t[0] as i64);
        mdf.time_stamp = t[0];
        mdf.date_time = dt.format("%Y-%m-%d %H:%M:%S%.9f").to_string();
        mdf.first_dg_offset = fisrt_dg_offset;       
        Ok(())
    }
    
    pub fn get_child_link_list(file: &mut BufReader<File>, first_child_offset: u64, block_type: &'static str) 
        -> Result<Vec<BlockInfo>, Box<dyn std::error::Error>> {
        let mut link_list: Vec<BlockInfo> = Vec::new();
        let blk_str: String = block_type.to_lowercase();
        let block_desc: &BlockDesc = DESC_MAP.get(block_type).unwrap();
        let link_name = format!("{0}_{0}_next", blk_str);   // there is a pattern for CN CG DG link-list
        let mut cursor = first_child_offset;
        let mut counter = 0;
        loop {
            let blk: BlockInfo = block_desc.try_parse_buf(file, cursor)?;
            cursor = blk.get_link_offset(link_name.as_str()).unwrap();
            link_list.push(blk);
            if cursor == 0 {
                break;
            }
            counter += 1; 
            if counter > 1000 {
                panic!("too many blocks in list at offset: {}", first_child_offset);
            }
        }
        Ok(link_list)
    }

}


#[cfg(test)]
pub mod parser_test {
    use std::io::BufReader;
    use crate::parser::parser::*;

    #[test]
    fn test_parse_toml() {
        let block = parse_toml("dg").unwrap();
        assert!(block.check_id("##DG".as_bytes()));
    }

    #[test]
    fn test_parse_mdf_header() {
        let file = std::fs::File::open("test/1.mf4").unwrap();
        let mut buf = BufReader::new(file);
        let mut mdf = MdfInfo::new(); 
        parse_mdf_header(&mut buf, &mut mdf).unwrap();
        assert_eq!(mdf.version, "4.10".to_string());
        assert_eq!(mdf.version_num, 410);
        assert_eq!(mdf.first_dg_offset, 0x8db0);
        let block = get_block_desc(&mut buf, mdf.first_dg_offset).unwrap();
        assert!(block.check_id("##DG".as_bytes()));
    }

    #[test]
    fn test_get_block_desc() {
        let file = std::fs::File::open("test/1.mf4").unwrap();
        let mut buf = BufReader::new(file);
        let block = get_block_desc(&mut buf, 0x8db0).unwrap();
        assert!(block.check_id("##DG".as_bytes()));
        let block = get_block_desc(&mut buf, 0x40).unwrap();
        assert!(block.check_id("##HD".as_bytes()));
    }

    #[test]
    fn test_get_child_link_list() {
        let file: std::fs::File = std::fs::File::open("test/1.mf4").unwrap();
        let mut buf: BufReader<std::fs::File> = BufReader::new(file);
        let mut mdf: MdfInfo = MdfInfo::new();
        parse_mdf_header(&mut buf, &mut mdf).unwrap();
        let link_list: Vec<mf4_parse::block::BlockInfo> = get_child_link_list(&mut buf, 
                                                                mdf.first_dg_offset, "DG").unwrap();
        println!("Total DG count: {}", link_list.len());
        for blk in link_list.iter() {
            println!("{:?}", blk);
        }
        // get the children of first dg 
        let cg_list = get_child_link_list(&mut buf,
             link_list[0].get_link_offset("dg_cg_first").unwrap(), "CG").unwrap();
            
        println!("Total CG count: {}", cg_list.len());
        for cg in cg_list.iter() {
            println!("{:?}", cg);
        }
    }

    #[test]
    fn test_parse_tx_block() {
        let file: std::fs::File = std::fs::File::open("test/1.mf4").unwrap();
        let mut buf: BufReader<std::fs::File> = BufReader::new(file);
        let block_desc = get_block_desc(&mut buf, 0x8e30).unwrap();
        println!("{:?}", block_desc);
        let block_info = block_desc.try_parse_buf(&mut buf, 0x8e30).unwrap();
        let ss:String = block_info.get_data_value("tx_data").unwrap().to_owned().try_into().unwrap();
        println!("info:::::::{:?}", ss);
    }
}