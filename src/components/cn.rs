pub mod channel {
    use std::io::{Cursor, BufReader, Read};
    use std::fs::File;
    use crate::block::BlockDesc;
    use crate::parser::{get_clean_text, get_block_desc_by_name};
    use crate::components::cc::conversion::Conversion;
    use crate::components::si::sourceinfo::SourceInfo;
    use std::fmt::Display;
    use crate::data_serde::{FromBeBytes, FromLeBytes};

    

    #[derive(Debug, Clone)]
    enum SyncType {
        None,
        Time,
        Angle,
        Distance,
        Index,
    }
    #[derive(Debug, Clone)]
    pub struct Channel {
        name: String,
        source: SourceInfo,
        conversion: Conversion,
        unit: String,
        comment: String,
        cn_type: u8,
        sync_type: SyncType,
        data_type: u8,
        bit_offset: u8,
        byte_offset: u32,
        bit_count: u32,
    }

    impl Channel {
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, Box<dyn std::error::Error>> {
            let desc: &BlockDesc = get_block_desc_by_name("CN".to_string()).expect("CN block not found");
            let info = desc.try_parse_buf(buf, offset).unwrap();
            let name = get_clean_text(buf, info.get_link_offset_normal("cn_tx_name").unwrap())?;
            let source = SourceInfo::new(buf, info.get_link_offset_normal("cn_si_source").unwrap())?;
            let conversion = Conversion::new(buf, info.get_link_offset_normal("cn_cc_conversion").unwrap())?;
            let unit = get_clean_text(buf, info.get_link_offset_normal("cn_md_unit").unwrap())?;
            let comment = get_clean_text(buf, info.get_link_offset_normal("cn_md_comment").unwrap())?;
            let cn_type: u8 = info.get_data_value_first("cn_type").ok_or("cn_type not found")?;
            let sync_type = match info.get_data_value_first::<u8>("cn_sync_type") {
                Some(0) => SyncType::None,
                Some(1) => SyncType::Time,
                Some(2) => SyncType::Angle,
                Some(3) => SyncType::Distance,
                Some(4) => SyncType::Index,
                _ => return Err("cn_sync_type not found".into()),
            };
            let data_type: u8 = info.get_data_value_first("cn_data_type").ok_or("cn_data_type not found")?;
            let bit_offset = info.get_data_value_first("cn_bit_offset").ok_or("cn_bit_offset not found")?;
            let byte_offset = info.get_data_value_first("cn_byte_offset").ok_or("cn_byte_offset not found")?;
            let bit_count = info.get_data_value_first("cn_bit_count").ok_or("cn_bit_count not found")?;
            Ok(Self {
                name,
                source,
                conversion,
                unit,
                comment,
                cn_type,
                sync_type,
                data_type,
                bit_offset,
                byte_offset,
                bit_count,
            })
        }

        pub fn get_name(&self) -> &str {
            &self.name
        }

        pub fn get_source(&self) -> &SourceInfo {
            &self.source
        }

        pub fn get_conversion(&self) -> &Conversion {
            &self.conversion
        }

        pub fn get_unit(&self) -> &str {
            &self.unit
        }

        pub fn get_comment(&self) -> &str {
            &self.comment
        }

        pub fn get_bit_size(&self) -> u32 {
            self.bit_count
        }

        pub fn get_byte_offset(&self) -> u32 {
            self.byte_offset
        }

        pub fn get_bit_offset(&self) -> u8 {
            self.bit_offset
        }

        pub fn from_bytes<T>(self, bytes: Vec<u8>) -> Result<Vec<T>, Box<dyn std::error::Error>> 
        where T: FromBeBytes + FromLeBytes
        {
            // this function will consume bytes
            let buf = Cursor::new(bytes);
            let mut res: Vec<T> = Vec::new();
            
            Ok(Vec::new())
        }
    }

    impl Display for Channel {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Channel: {}", self.name)?;
            write!(f, "\n-----------Source: {}", self.source)?;
            write!(f, "\n-----------ChannelType: {}", self.cn_type)?;
            write!(f, "\n-----------SyncType: {:?}", self.sync_type)?;
            write!(f, "\n-----------Conversion: {}: {:?}", self.conversion.get_cc_name(), self.conversion.get_cc_type())?;
            write!(f, "\n-----------Unit: {}", self.unit)?;
            write!(f, "\n-----------DataType: {}", self.data_type)?;
            write!(f, "\nEND Channel {}", self.name)
        }
    }
}

