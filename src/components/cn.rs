pub mod channel {
    use std::io::BufReader;
    use std::fs::File;
    use crate::block::BlockDesc;
    use crate::parser::{get_text, get_block_desc_by_name};
    use crate::components::cc::conversion::Conversion;
    use crate::components::si::sourceinfo::SourceInfo;

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
            let name = get_text(buf, info.get_link_offset_normal("cn_tx_name").unwrap())?;
            let source = SourceInfo::new(buf, info.get_link_offset_normal("cn_si_source").unwrap())?;
            let conversion = Conversion::new(buf, info.get_link_offset_normal("cn_cc_conversion").unwrap())?;
            let unit = get_text(buf, info.get_link_offset_normal("cn_md_unit").unwrap())?;
            let comment = get_text(buf, info.get_link_offset_normal("cn_md_comment").unwrap())?;
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


    }
}

