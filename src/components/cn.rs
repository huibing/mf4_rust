pub mod channel {
    use std::io::{Cursor, BufReader};
    use std::fs::File;
    use std::fmt::Display;
    use crate::block::BlockDesc;
    use crate::parser::{get_clean_text, get_block_desc_by_name};
    use crate::components::cc::conversion::Conversion;
    use crate::components::si::sourceinfo::SourceInfo;
    use crate::components::dg::datagroup::DataGroup;
    use crate::data_serde::{FromBeBytes, FromLeBytes, right_shift_bytes, bytes_and_bits};

    
    type DynError = Box<dyn std::error::Error>;
    #[derive(Debug, Clone)]
    pub enum SyncType {
        None,
        Time,
        Angle,
        Distance,
        Index,
    }

    #[derive(Debug, Clone)]
    enum CnDataType{
        LeUnsigned,
        LeSigned,
        BeUnsigned,
        BeSigned,
        LeFloat,
        BeFloat,
        Utf8Str,
        BeUtf16Str,
        LeUtf16Str,
        NotImplemented
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
        cn_data_type: CnDataType,
        bit_offset: u8,
        byte_offset: u32,
        bit_count: u32,
        bytes_num: u32,
        master: bool,
    }

    impl Channel {
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, DynError> {
            let desc: &BlockDesc = get_block_desc_by_name("CN".to_string()).expect("CN block not found");
            let info: crate::block::BlockInfo = desc.try_parse_buf(buf, offset).unwrap();
            let name = get_clean_text(buf, info.get_link_offset_normal("cn_tx_name").unwrap())?;
            let source = SourceInfo::new(buf, info.get_link_offset_normal("cn_si_source").unwrap())?;
            let conversion = Conversion::new(buf, info.get_link_offset_normal("cn_cc_conversion").unwrap())?;
            let unit: String = get_clean_text(buf, info.get_link_offset_normal("cn_md_unit").unwrap())
                                .unwrap_or("".to_string());
            let comment: String = get_clean_text(buf, info.get_link_offset_normal("cn_md_comment").unwrap())
                                .unwrap_or("".to_string());
            let cn_type: u8 = info.get_data_value_first("cn_type").ok_or("cn_type not found")?;
            let master = match cn_type {
                0 => false,
                2 | 3 => true,
                _ => return Err("cn_type not supportted yet.".into()),
            };
            let sync_type: SyncType = match info.get_data_value_first::<u8>("cn_sync_type") {
                Some(0) => SyncType::None,
                Some(1) => SyncType::Time,
                Some(2) => SyncType::Angle,
                Some(3) => SyncType::Distance,
                Some(4) => SyncType::Index,
                _ => return Err("cn_sync_type not found".into()),
            };
            let data_type: u8 = info.get_data_value_first("cn_data_type").ok_or("cn_data_type not found")?;
            let bit_offset: u8 = info.get_data_value_first("cn_bit_offset").ok_or("cn_bit_offset not found")?;
            let byte_offset: u32 = info.get_data_value_first("cn_byte_offset").ok_or("cn_byte_offset not found")?;
            let bit_count: u32 = info.get_data_value_first("cn_bit_count").ok_or("cn_bit_count not found")?;
            let bytes_num: u32 = (bit_count as f32 / 8.0).ceil() as u32;
            let cn_data_type = match data_type {
                0 => CnDataType::LeUnsigned,
                1 => CnDataType::BeUnsigned,
                2 => CnDataType::LeSigned,
                3 => CnDataType::BeSigned,
                4 => CnDataType::LeFloat,
                5 => CnDataType::BeFloat,
                6|7 => CnDataType::Utf8Str,
                8 => CnDataType::LeUtf16Str,
                9 => CnDataType::BeUtf16Str,
                _ => CnDataType::NotImplemented,
            };
            Ok(Self {
                name,
                source,
                conversion,
                unit,
                comment,
                cn_type,
                sync_type,
                data_type,
                cn_data_type,
                bit_offset,
                byte_offset,
                bit_count,
                master,
                bytes_num,
            })
        }

        pub fn get_name(&self) -> &str {
            &self.name
        }

        pub fn get_sync_type(&self) -> &SyncType {
            &self.sync_type
        }

        pub fn get_cn_type(&self) -> &u8 {
            &self.cn_type
        }

        pub fn get_data_type(&self) -> u8 {
            self.data_type
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

        pub fn from_bytes<T>(&self, rec_bytes: &Vec<u8>) -> Result<T, DynError> 
        where T: FromBeBytes + FromLeBytes
        {
            let bytes_to_read = self.bytes_num;
            let raw_data = rec_bytes[self.byte_offset as usize..
                                (self.byte_offset + bytes_to_read) as usize].to_vec();
                // TODO: handle bit offset
            let cn_data = if self.bit_offset != 0 {
                let mut new_bytes = right_shift_bytes(&raw_data, self.bit_offset)?;
                bytes_and_bits(&mut new_bytes, self.bit_count);
                new_bytes
            } else {
                raw_data
            };
            let mut data_buf = Cursor::new(cn_data);
            match self.data_type {
                // only distinguish little-edian and big-endian here. Concrete data types are handled in the up
                // level functions.
                0|2|4|6|7|8 => Ok(T::from_le_bytes(&mut data_buf)),
                1|3|5|9 => Ok(T::from_be_bytes(&mut data_buf)),
                _ => Err("data type not supportted.".into()),
            }
        }

        pub fn is_master(&self) -> bool {
            self.master
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

