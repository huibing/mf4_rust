
pub mod block {  // utility struct and functions for parsing mdf block link and data
    use serde::{Deserialize, Serialize};
    use toml::Value;
    use byteorder::{ByteOrder, LittleEndian};
    use std::io::BufReader;
    use std::fs::File;
    use std::io::{Seek, SeekFrom, Read, Cursor};
    use indexmap::IndexMap;
    use std::convert::{TryInto, TryFrom};
    use crate::parser::get_block_desc_by_name;
    

    #[derive(Serialize, Deserialize, Debug)]
    pub struct BlockField {
        data_type: DataType,
        size: Option<u32>,   //TODO: if NONE, means size is variable
    }
    impl BlockField {
        pub fn get_data_type(&self) -> String {
            match self.data_type {
                DataType::BYTE => "BYTE".to_string(),
                DataType::CHAR => "CHAR".to_string(),
                DataType::UINT64 => "UINT64".to_string(),
                DataType::UINT8 => "UINT8".to_string(),
                DataType::INT16 => "INT16".to_string(),
                DataType::INT32 => "INT32".to_string(),
                DataType::UINT16 => "UINT16".to_string(),
                DataType::UINT32 => "UINT32".to_string(),
                DataType::INT64 => "INT64".to_string(),
                DataType::REAL => "REAL".to_string(),
            }
        }

        pub fn try_parse_value(&self, cur: &mut Cursor<&Vec<u8>>) -> Result<DataValue, Box<dyn std::error::Error>> {
            let size:usize = self.size.unwrap_or(1).try_into().unwrap();  // convert to usize for later convience
            match self.data_type {
                DataType::CHAR => {
                    let size:usize;
                    if self.size.is_none() {
                        size = cur.get_ref().len() - cur.position() as usize;
                    } else {
                        size = self.size.unwrap() as usize;
                    }
                    let mut byte_buf: Vec<u8> = vec![0u8;size];
                    cur.read_exact(&mut byte_buf)?;
                    Ok(DataValue::CHAR(String::from_utf8(byte_buf)?))   // might be wrong, asam manual says that CHAR data is encoded in ISO-8859-1
                },
                DataType::UINT8 => {
                    let mut byte_buf: Vec<u8> = vec![0u8;size];
                    cur.read_exact(&mut byte_buf)?;
                    Ok(DataValue::UINT8(byte_buf))
                },
                DataType::BYTE => {
                    let mut byte_buf: Vec<u8> = vec![0u8;size];
                    cur.read_exact(&mut byte_buf)?;
                    Ok(DataValue::BYTE(byte_buf))
                },
                DataType::UINT64 => {
                    let mut res: Vec<u64> = Vec::new();
                    let mut eight_bytes_buf: [u8; 8] = [0u8;8];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut eight_bytes_buf).unwrap();
                        res.push(LittleEndian::read_u64(&eight_bytes_buf));
                    });
                    Ok(DataValue::UINT64(res))
                },
                DataType::INT16 => {
                    let mut res: Vec<i16> = Vec::new();
                    let mut two_byte_buf: [u8; 2] = [0u8;2];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut two_byte_buf).unwrap();
                        res.push(LittleEndian::read_i16(&two_byte_buf));
                    });
                    Ok(DataValue::INT16(res))
                },
                DataType::UINT16 => {
                    let mut res: Vec<u16> = Vec::new();
                    let mut two_byte_buf = [0u8;2];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut two_byte_buf).unwrap();
                        res.push(LittleEndian::read_u16(&two_byte_buf));
                    });
                    Ok(DataValue::UINT16(res))
                },
                DataType::INT32 => {
                    let mut res: Vec<i32> = Vec::new();
                    let mut buf: [u8; 4] = [0u8;4];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_i32(&buf));
                    });
                    Ok(DataValue::INT32(res))
                },
                DataType::UINT32 => {
                    let mut res: Vec<u32> = Vec::new();
                    let mut buf: [u8; 4] = [0u8;4];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_u32(&buf));
                    });
                    Ok(DataValue::UINT32(res))
                },
                DataType::INT64 => {
                    let mut res: Vec<i64> = Vec::new();
                    let mut buf: [u8; 8] = [0u8;8];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_i64(&buf));
                    });
                    Ok(DataValue::INT64(res))
                },
                DataType::REAL => {
                    let mut res: Vec<f64> = Vec::new();
                    let mut buf: [u8; 8] = [0u8;8];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_f64(&buf));
                    });
                    Ok(DataValue::REAL(res))
                },
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    pub enum DataValue {
        CHAR(String),
        BYTE(Vec<u8>),
        UINT64(Vec<u64>),
        UINT8(Vec<u8>),
        INT16(Vec<i16>),
        UINT16(Vec<u16>),
        INT32(Vec<i32>),
        UINT32(Vec<u32>), 
        INT64(Vec<i64>),
        REAL(Vec<f64>),
    }

    impl TryFrom<DataValue> for String {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::CHAR(s) => Ok(s),
                _ => Err("DataValue is not a CHAR")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<u8> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::BYTE(s) => Ok(s),
                DataValue::UINT8(s) => Ok(s),
                _ => Err("DataValue is not a uint8 or byte")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<u64> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::UINT64(s) => Ok(s),
                _ => Err("DataValue is not a uint64")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<i16> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::INT16(s) => Ok(s),
                _ => Err("DataValue is not a int16")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<u16> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::UINT16(s) => Ok(s),
                _ => Err("DataValue is not a uint16")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<i32> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::INT32(s) => Ok(s),
                _ => Err("DataValue is not a int32")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<u32> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::UINT32(s) => Ok(s),
                _ => Err("DataValue is not a uint32")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<i64> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::INT64(s) => Ok(s),
                _ => Err("DataValue is not a int64")
            }
        }
    }

    impl TryFrom<DataValue> for Vec<f64> {
        type Error = &'static str;
        fn try_from(value: DataValue) -> Result<Self, Self::Error> {
            match value {
                DataValue::REAL(s) => Ok(s),
                _ => Err("DataValue is not a float64")
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    enum DataType {
        CHAR,
        BYTE,
        UINT64,
        UINT8,
        INT16, 
        UINT16,
        INT32,
        UINT32,
        INT64,
        REAL,   // double precision float 
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct BlockDesc {
        id: String,
        implemented: Option<bool>,   // if None, means implemented by default
        link: Value,
        data: Value
    }

    impl BlockDesc {      
        pub fn get_data_fields(&self) -> Option<Vec<&String>> {
            match &self.data {
                Value::Table(table) => {
                    Some(table.keys().collect())
                },
                _ => {
                    None
                }
            }
        }

        pub fn get_link_fields(&self) -> Option<Vec<&String>> {
            match &self.link {
                Value::Table(table) => {
                    Some(table.keys().collect())
                },
                _ => {
                    None
                }
            }
        }

        pub fn is_implemented(&self) -> bool {
            if let Some(implemented) = &self.implemented {
                *implemented
            } else {
                true
            }
        }
        
        pub fn check_id(&self, id: &[u8]) -> bool {
            let id_array = self.id.as_bytes();
            id_array == id
        }

        pub fn get_data_field(&self, field_name: &str) -> Option<BlockField> {
            match &self.data {
                Value::Table(table) => {
                    Some(table.get(field_name)?.clone().try_into().unwrap())
                },
                _ => {
                    None
                }
            }
        }

        pub fn get_link_block_type(&self, link_name: &str) -> Option<Vec<String>> {
            match &self.link {
                Value::Table(table) => {
                    Some(table.get(link_name)?.clone().try_into::<Vec<String>>().unwrap())
                },
                _ => {
                    None
                }
            }
        }

        pub fn try_parse_buf(&self, buf: &mut BufReader<File>, offset: u64) -> Result<BlockInfo, Box<dyn std::error::Error>>{
            // read id
            buf.seek(SeekFrom::Start(offset)).unwrap();
            let mut id_buf = [0u8;4];
            buf.read_exact(&mut id_buf).unwrap();
            if !self.check_id(&id_buf) {
                println!("Invalid block id");     // TODO: debug info  put into logger
                return Err("Invalid block id".into());
            } else {
                let mut blk_info: BlockInfo = BlockInfo {
                    links: Vec::new(),
                    data: IndexMap::new(),
                    id: self.id.to_owned(),
                    link_map: IndexMap::new()
                };
                // read 20 more bytes
                let mut data_buf: [u8; 20] = [0u8;20];
                buf.read_exact(&mut data_buf).unwrap();
                // parse length and link count
                let blk_len = LittleEndian::read_u64(&data_buf[4..12]);
                let link_count: u64 = LittleEndian::read_u64(&data_buf[12..20]);
                // decide to read how many bytes using blk_len
                let mut vec_buf: Vec<u8> = vec![0u8;blk_len as usize -24];
                buf.read_exact(&mut vec_buf).unwrap();
                let mut cur = Cursor::new(&vec_buf);
                if link_count > 0 {
                    let mut link_buf: [u8; 8] = [0u8;8];   // link are 8 bytes long 
                    // it is very important that link fields are ordered just like in toml file
                    for _ in 0..link_count {
                        //zip to make sure not to exceed link count, toml config has some link are OPTIONAL
                        cur.read_exact(&mut link_buf).unwrap();
                        let link_offset = LittleEndian::read_u64(&link_buf);
                        blk_info.links.push(link_offset);
                    }
                }
                // read parse data using datafield's try_parse_value method
                for dname in self.get_data_fields().unwrap() {
                    let field = self.get_data_field(dname).unwrap();  // panic if no field description found
                    let data_value = field.try_parse_value(&mut cur)?;
                    blk_info.data.insert(dname.clone(), data_value);
                }
                blk_info.map_links().unwrap();
                Ok(blk_info)
            }
        }

    }
    
    #[derive(Debug)]
    pub struct BlockInfo {
        pub links: Vec<u64>,
        pub data: IndexMap<String, DataValue>,
        id: String,
        link_map: IndexMap<String, LinkAddr>
    }

    #[derive(Debug, Clone)]
    enum LinkAddr {
        Normal(u64),
        Variable(Vec<u64>)
    }

    impl TryFrom<LinkAddr> for u64 {
        type Error = &'static str;
        fn try_from(value: LinkAddr) -> Result<Self, Self::Error> {
            if let LinkAddr::Normal(num) = value {
                Ok(num)
            } else {
                Err("LinkAddr is not a normal link")
            }
        }
    }

    impl TryFrom<LinkAddr> for Vec<u64> {
        type Error = &'static str;
        fn try_from(value: LinkAddr) -> Result<Self, Self::Error> {
            if let LinkAddr::Variable(num) = value {
                Ok(num)
            } else {
                Err("LinkAddr is not a variable link")
            }
        }
    }

    impl BlockInfo {
        pub fn map_links(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            match self.id.as_str() {
                "##CA" | "##LD" => {  // these blocks have variable size links
                    Err("Not Implemented".into())
                }
                "##CN" => {  // these blocks have variable size links
                    let at_ref_nums:Vec<u16> = self.data.get(&"cn_attachment_count".to_string()).
                                                ok_or("can not find cn_attachment_count")?.clone().try_into()?;
                    let cn_flags:Vec<u32> = self.data.get(&"cn_flags".to_string()).
                                                ok_or("can not find cn_flags")?.clone().try_into()?;
                    let default_x_flag: bool = cn_flags[0] & (0x01<<12) == (0x01<<12);
                    //collect normal link first
                    let cn_desc = get_block_desc_by_name("CN".to_string()).unwrap();
                    let cn_link_fields = cn_desc.get_link_fields().ok_or("can not find cn_link_fields")?;
                    let mut i = 0;
                    for name in cn_link_fields {
                        let link_addr = self.links[i];
                        self.link_map.insert(name.clone(), LinkAddr::Normal(link_addr));
                        i += 1;
                    }
                    //collect variable link 
                    if at_ref_nums[0]  > 0 {
                        if i + at_ref_nums[0] as usize > self.links.len() {
                            return Err("Invalid link count".into());
                        }
                        let mut link_vec: Vec<u64> = Vec::new();
                        (0..at_ref_nums[0]).for_each(|_| {
                            link_vec.push(self.links[i]);
                            i += 1;
                        });
                        self.link_map.insert("cn_attachment_first".to_string(), LinkAddr::Variable(link_vec));
                    }
                    if default_x_flag {
                        if i + 3 > self.links.len() {
                            return Err("Invalid link count".into());
                        }
                        let mut link_vec: Vec<u64> = Vec::new();
                        (0..3).for_each(|_| {
                            link_vec.push(self.links[i]);  
                            i += 1;
                        });
                        self.link_map.insert("cn_default_x".to_string(), LinkAddr::Variable(link_vec));
                    }
                    Ok(())
                }
                id => {// This is the normal case
                    let block_type:String = id[2..].to_string();
                    let block_desc: &BlockDesc = get_block_desc_by_name(block_type).unwrap();
                    for (i, name) in (0..self.links.len()).zip(block_desc.get_link_fields().unwrap()) {
                        let link_addr = self.links[i];
                        self.link_map.insert(name.to_owned(), LinkAddr::Normal(link_addr));
                    }
                    Ok(())
                }
            }
        }
        pub fn get_link_offset_normal(&self, link_name: &str) -> Option<u64> {
            Some(self.link_map.get(&link_name.to_string())?.clone().try_into().unwrap())
        }
        pub fn get_link_offset_vec(&self, link_name: &str) -> Option<Vec<u64>> {
            Some(self.link_map.get(&link_name.to_string())?.clone().try_into().unwrap())
        }
        pub fn get_data_value(&self, data_name: &str) -> Option<&DataValue> {
            Some(self.data.get(data_name)?)
        }
        pub fn get_data_value_copy(&self, data_name: &str) -> Option<DataValue> {
            Some(self.data.get(data_name)?.clone())
        }
        pub fn get_id(&self) -> &String {
            &self.id
        }
    }
}

pub mod parser {
    use crate::block::{BlockDesc, BlockInfo};
    use rust_embed::RustEmbed;
    use std::io::{BufReader, Seek, Read, SeekFrom};
    use std::path::PathBuf;
    use std::fs::File;
    use byteorder::{LittleEndian, ByteOrder};
    use std::collections::HashMap;
    use chrono::DateTime;
    use lazy_static::lazy_static;


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
        pub static ref DESC_MAP: HashMap<String, BlockDesc> = {
            let mut m = HashMap::new();
            let block_types = ["DG", "HD", "CG", "TX", "MD", "CN"];
            block_types.into_iter().for_each(|key| {
                let desc = parse_toml(key.to_lowercase().as_str()).unwrap();  // toml file names in lowercase
                m.insert(key.to_string(), desc);  // key in uppercase
            });
            m
        };
    }
    pub fn get_block_desc_by_name(name: String) -> Option<&'static BlockDesc> {
        Some(DESC_MAP.get(&name)?)
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
        if mdf.version_num < 400 {
            panic!("unsupported version: {}", mdf.version_num);   // do not support any version below 4.00
        }
        file.seek(SeekFrom::Current(30))?; // skip 30 bytes
        file.read_exact(&mut two_bytes)?; //id_unfin_flags
        file.read_exact(&mut two_bytes)?; //id_custom_unfin_flags
        let offset = file.stream_position().unwrap();
        //parse header HD block
        let block: &BlockDesc = get_block_desc(file, 0x40)?;
        let header_info: BlockInfo = block.try_parse_buf(file, offset)?;
        let fisrt_dg_offset: u64 = header_info.get_link_offset_normal("hd_dg_first").unwrap();
        //parse time stamp
        let time_stamp_v = header_info.get_data_value("hd_start_time_ns").unwrap();
        let t: Vec<u64> = time_stamp_v.clone().try_into().unwrap();
        let dt = DateTime::from_timestamp_nanos(t[0] as i64);
        mdf.time_stamp = t[0];
        mdf.date_time = dt.format("%Y-%m-%d %H:%M:%S%.9f").to_string();
        mdf.first_dg_offset = fisrt_dg_offset;       
        Ok(())
    }
    
    pub fn get_child_link_list<'a>(file: &mut BufReader<File>, first_child_offset: u64, block_type: &'static str) 
        -> Result<Vec<BlockInfo>, Box<dyn std::error::Error>> {
        let mut link_list: Vec<BlockInfo> = Vec::new();
        let blk_str: String = block_type.to_lowercase();
        let block_desc: &BlockDesc = DESC_MAP.get(block_type).unwrap();
        let link_name = format!("{0}_{0}_next", blk_str);   // there is a pattern for CN CG DG link-list
        let mut cursor = first_child_offset;
        let mut counter = 0;
        loop {
            let blk: BlockInfo = block_desc.try_parse_buf(file, cursor)?;
            cursor = blk.get_link_offset_normal(link_name.as_str()).unwrap();
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

    pub fn cn_get_name(file: &mut BufReader<File>, cn_info: &BlockInfo) -> Result<String, Box<dyn std::error::Error>> {
        let name_offset = cn_info.get_link_offset_normal("cn_tx_name").unwrap();
        let desc = get_block_desc(file, name_offset)?;
        let name_info: BlockInfo = desc.try_parse_buf(file, name_offset)?;
        match name_info.get_id().as_str() {
            "##TX" => {
                let name_v = name_info.get_data_value("tx_data").unwrap();
                Ok(name_v.clone().try_into().unwrap())
            },
            "##MD" => {
                let name_v = name_info.get_data_value("md_data").unwrap();
                Ok(name_v.clone().try_into().unwrap())
            },
            _ => {
                Err("unknown CN name block".into())
            },
        }   
    }
        
}

#[cfg(test)]
pub mod test_block {
    use crate::block::*;
    use std::{fs::{self, File}, io::{BufReader, Write}};
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "test/"]
    #[prefix = "test/"]
    struct Asset;

    #[test]
    fn test_block_parse() {
        let dg_toml_file = Asset::get("test/dg.toml").unwrap();
        let toml_str = String::from_utf8(dg_toml_file.data.as_ref().to_vec()).unwrap();
        let block: BlockDesc = toml::from_str(toml_str.as_str()).unwrap();
        let file_data = Asset::get("test/1.mf4").unwrap();
        let mut new_file = File::create("temp.mf4").unwrap();
        new_file.write(file_data.data.as_ref()).unwrap();
        let file = File::open("temp.mf4").unwrap();
        let mut buf = BufReader::new(file);
        let block_info = block.try_parse_buf(&mut buf, 0x8db0).unwrap();  // one DG block starts at offset 992 in test_mdf.mf4 file
        assert_eq!(block_info.links.len(), 4);
        assert_eq!(block_info.data.len(), 2);
        assert_eq!(block_info.get_link_offset_normal("dg_dg_next").unwrap(), 36144);
        assert_eq!(block_info.get_link_offset_normal("dg_cg_first").unwrap(), 25600);
        assert_eq!(block_info.get_link_offset_normal("dg_data").unwrap(), 49712);

        let data_value = block_info.data.get("dg_rec_id_size").unwrap();
        if let DataValue::UINT8(vec) = data_value {
            assert_eq!(vec.len(), 1);
            assert_eq!(vec[0], 0);
        } else {
            assert!(false, "data value is not UINT8");
        }
        let data_value = block_info.data.get("dg_reserved").unwrap();
        if let DataValue::BYTE(vec) = data_value {
            assert_eq!(vec.len(), 7);
            assert_eq!(vec, &vec![0;7]);
        } else {
            assert!(false, "data value is not BYTE");
        }

        assert_eq!(block_info.get_data_value("dg_rec_id_size1"), None);
        assert_eq!(block_info.get_data_value("dg_rec_id_size").unwrap(), &DataValue::UINT8(vec![0]));
        fs::remove_file("temp.mf4").unwrap();
    }
}

#[cfg(test)]
pub mod parser_test {
    use std::io::BufReader;
    use crate::parser::*;
    use crate::block::*;

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
        let link_list: Vec<BlockInfo> = get_child_link_list(&mut buf, 
                                                                mdf.first_dg_offset, "DG").unwrap();
        println!("Total DG count: {}", link_list.len());
        for blk in link_list.iter() {
            println!("{:?}", blk);
        }
        // get the children of first dg 
        let cg_list = get_child_link_list(&mut buf,
             link_list[0].get_link_offset_normal("dg_cg_first").unwrap(), "CG").unwrap();
            
        println!("Total CG count: {}", cg_list.len());
        for cg in cg_list.iter() {
            println!("{:?}", cg);
        }

        let cn_list = get_child_link_list(&mut buf, cg_list[0].get_link_offset_normal("cg_cn_first").unwrap(), "CN").unwrap();
        println!("Total CN count: {}", cn_list.len());
        for cn in cn_list.iter() {
            println!("{:?}", cn);
            println!("CN name: {}", cn_get_name(&mut buf, cn).unwrap());
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