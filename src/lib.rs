pub mod block {  // utility struct and functions for parsing mdf block data
    use serde::{Deserialize, Serialize};
    use toml::Value;
    use byteorder::{ByteOrder, LittleEndian};
    use std::io::BufReader;
    use std::fs::File;
    use std::io::{Seek, SeekFrom, Read, Cursor};
    use indexmap::IndexMap;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct BlockField {
        data_type: DataType,
        size: Option<u32>,
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
                    let mut byte_buf = vec![0u8;size];
                    cur.read_exact(&mut byte_buf)?;
                    Ok(DataValue::CHAR(String::from_utf8(byte_buf)?))   // might be wrong, asam manual says that BYTE data is encoded in ISO-8859-1
                },
                DataType::UINT8 => {
                    let mut byte_buf = vec![0u8;size];
                    cur.read_exact(&mut byte_buf)?;
                    Ok(DataValue::UINT8(byte_buf))
                },
                DataType::BYTE => {
                    let mut byte_buf = vec![0u8;size];
                    cur.read_exact(&mut byte_buf)?;
                    Ok(DataValue::BYTE(byte_buf))
                },
                DataType::UINT64 => {
                    let mut res: Vec<u64> = Vec::new();
                    let mut eight_bytes_buf = [0u8;8];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut eight_bytes_buf).unwrap();
                        res.push(LittleEndian::read_u64(&eight_bytes_buf));
                    });
                    Ok(DataValue::UINT64(res))
                },
                DataType::INT16 => {
                    let mut res: Vec<i16> = Vec::new();
                    let mut two_byte_buf = [0u8;2];
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
                    let mut buf = [0u8;4];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_i32(&buf));
                    });
                    Ok(DataValue::INT32(res))
                },
                DataType::UINT32 => {
                    let mut res: Vec<u32> = Vec::new();
                    let mut buf = [0u8;4];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_u32(&buf));
                    });
                    Ok(DataValue::UINT32(res))
                },
                DataType::INT64 => {
                    let mut res: Vec<i64> = Vec::new();
                    let mut buf = [0u8;8];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_i64(&buf));
                    });
                    Ok(DataValue::INT64(res))
                },
                DataType::REAL => {
                    let mut res: Vec<f64> = Vec::new();
                    let mut buf = [0u8;8];
                    (0..size).into_iter().for_each(|_| {
                        cur.read_exact(&mut buf).unwrap();
                        res.push(LittleEndian::read_f64(&buf));
                    });
                    Ok(DataValue::REAL(res))
                },
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
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

    #[derive(Serialize, Deserialize, Debug)]
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
                let mut blk_info = BlockInfo {
                    links: IndexMap::new(),
                    data: IndexMap::new()
                };
                // read 20 more bytes
                let mut data_buf = [0u8;20];
                buf.read_exact(&mut data_buf).unwrap();
                // parse length and link count
                let blk_len = LittleEndian::read_u64(&data_buf[4..12]);
                let link_count: u64 = LittleEndian::read_u64(&data_buf[12..20]);
                // decide to read how many bytes using blk_len
                let mut vec_buf = vec![0u8;blk_len as usize -24];
                buf.read_exact(&mut vec_buf).unwrap();
                let mut cur = Cursor::new(&vec_buf);
                if link_count > 0 {
                    let mut link_buf = [0u8;8];   // link are 8 bytes long 
                    // it is very important that link fields are ordered just like in toml file
                    for lname in self.get_link_fields().unwrap() { 
                        cur.read_exact(&mut link_buf).unwrap();
                        let link_offset = LittleEndian::read_u64(&link_buf);
                        blk_info.links.insert(lname.clone(), link_offset);
                    }
                }
                // read parse data using datafield's try_parse_value method
                for dname in self.get_data_fields().unwrap() {
                    let field = self.get_data_field(dname).unwrap();  // panic if no field description found
                    let data_value = field.try_parse_value(&mut cur)?;
                    blk_info.data.insert(dname.clone(), data_value);
                }
                Ok(blk_info)
            }
        }

    }
    
    #[derive(Debug)]
    pub struct BlockInfo {
        pub links: IndexMap<String, u64>,
        pub data: IndexMap<String, DataValue>,
    }

    impl BlockInfo {
        pub fn get_link_offset(&self, link_name: &str) -> Option<u64> {
            Some(self.links.get(link_name)?.clone())
        }

        pub fn get_data_value(&self, data_name: &str) -> Option<&DataValue> {
            Some(self.data.get(data_name)?)
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
        let file_data = Asset::get("test/test_mdf.mf4").unwrap();
        let mut new_file = File::create("temp.mf4").unwrap();
        new_file.write(file_data.data.as_ref()).unwrap();
        let file = File::open("temp.mf4").unwrap();
        let mut buf = BufReader::new(file);
        let block_info = block.try_parse_buf(&mut buf, 992).unwrap();  // one DG block starts at offset 992 in test_mdf.mf4 file
        assert_eq!(block_info.links.len(), 4);
        assert_eq!(block_info.data.len(), 2);
        assert_eq!(block_info.links.get("dg_dg_next").unwrap(), &0);
        assert_eq!(block_info.links.get("dg_cg_first").unwrap(), &888);
        assert_eq!(block_info.links.get("dg_data").unwrap(), &1736);
        assert_eq!(block_info.get_link_offset("dg_dg_next").unwrap(), 0);
        assert_eq!(block_info.get_link_offset("dg_cg_first").unwrap(), 888);
        assert_eq!(block_info.get_link_offset("dg_data").unwrap(), 1736);

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