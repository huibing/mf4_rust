pub mod parser {
    use mf4_parse::block::*;
    use rust_embed::RustEmbed;
    use std::io::{BufReader, Seek, Read, SeekFrom};
    use std::path::PathBuf;
    use std::fs::File;
    use std::str::FromStr;
    use byteorder::{LittleEndian, ByteOrder};
    use std::collections::HashMap;


    #[derive(RustEmbed)]
    #[folder = "config/"]
    #[prefix = "config/"]
    struct Asset;   // compile config file asset to binary

    pub struct MdfInfo {
        pub version: String,
        pub version_num: u16,
    }

    impl MdfInfo {
        pub fn new() -> Self{
            MdfInfo {
                version: "4.10".to_string(),
                version_num: 410,
            }
        }
    }

    lazy_static! {
        static ref DESC_MAP: HashMap<String, BlockDesc> = {
            let mut m = HashMap::new();
            let block_types = ["DG", "HD"];
            block_types.into_iter().for_each(|key| {
                let key_str = String::from_str(key).unwrap();
                let desc = parse_toml(key_str.to_lowercase().as_str()).unwrap();
                m.insert(key_str, desc);
            });
            m
        };
    }

    pub fn get_block_desc(file: &mut BufReader<File>, offset: u64) -> Result<&BlockDesc, Box<dyn std::error::Error>>{
        //use file offset to acquire the actual block type and its block desc
        let mut buf = [0u8;4];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buf)?;
        let block_type = String::from_utf8(buf[2..].to_vec())?;
        Ok(DESC_MAP.get(&block_type).unwrap())
    }

    pub fn parse_toml(block_name: &str) -> Result<BlockDesc, Box<dyn std::error::Error>> {
        let mut path = PathBuf::from("config/");
        path.push(block_name);
        path.set_extension("toml");
        let toml_file = Asset::get(path.to_str().ok_or("")?).ok_or("")?;
        Ok(toml::from_str(std::str::from_utf8(toml_file.data.as_ref())?)?)
    }

    pub fn parse_mdf_id_block(file: &mut BufReader<File>, mdf: &mut MdfInfo) -> Result<(), Box<dyn std::error::Error>>{
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
        Ok(())
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
    fn test_parse_mdf_id_block() {
        let file = std::fs::File::open("test/1.mf4").unwrap();
        let mut buf = BufReader::new(file);
        let mut mdf = MdfInfo::new(); 
        parse_mdf_id_block(&mut buf, &mut mdf).unwrap();
        assert_eq!(mdf.version, "4.10".to_string());
        assert_eq!(mdf.version_num, 410);
    }

    #[test]
    fn test_get_block_desc() {
        let file = std::fs::File::open("test/1.mf4").unwrap();
        let mut buf = BufReader::new(file);
        let block = get_block_desc(&mut buf, 0x8db0).unwrap();
        assert!(block.check_id("##DG".as_bytes()));
    }
}