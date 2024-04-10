pub mod datagroup {
    use crate::components::cg::channelgroup::ChannelGroup;
    use crate::components::cn::channel::Channel;
    use crate::parser::{get_block_desc_by_name, get_clean_text, get_child_links};
    use std::collections::HashMap;
    use std::io::{BufReader, Seek, SeekFrom, Read};
    use std::fs::File;
    use std::fmt::Display;

    type DynError = Box<dyn std::error::Error>;
    #[derive(Debug, Clone, Copy)]
    pub enum RecIDSize {
        NORECID,
        UINT8,
        UINT16,
        UINT32,
        UINT64,
    }

    #[derive(Debug)]
    pub struct ChannelLink<'a>
        (&'a Channel, &'a ChannelGroup, &'a DataGroup);

    impl ChannelLink<'_> {
        pub fn get_channel(&self) -> &Channel {
            &self.0
        }

        pub fn get_channel_group(&self) -> &ChannelGroup {
            &self.1
        }

        pub fn get_data_group(&self) -> &DataGroup {
            &self.2
        }

    }

    #[derive(Debug)]
    pub struct DataGroup{   
    // DataGroup is a container for ChannelGroups; this struct OWNS the ChannelGroups and its channels
        rec_id_size: RecIDSize,
        comment: String,
        data: u64,
        channel_groups: Vec<ChannelGroup>,
        sorted: bool,
        rec_id_map: HashMap<u64, (u32, u64)>, // record id -> (data bytes, cycle count)
        offsets_map: HashMap<u64, Vec<u64>>, // record id -> offset
    }

    fn read_rec_id(rec_id_size: RecIDSize, buf: &mut BufReader<File>) -> Result<u64, DynError> {
        // read record id to process ; Note this function will move buf's cursor
        match rec_id_size {
            RecIDSize::NORECID => Ok(0),
            RecIDSize::UINT8 => {
                let mut temp_buf = [0u8; 1];
                buf.read_exact(&mut temp_buf).unwrap();
                Ok(temp_buf[0] as u64)},
            RecIDSize::UINT16 => {
                let mut temp_buf = [0u8; 2];
                buf.read_exact(&mut temp_buf).unwrap();
                Ok(u16::from_le_bytes(temp_buf) as u64)
            },
            RecIDSize::UINT32 => {
                let mut temp_buf = [0u8; 4];
                buf.read_exact(&mut temp_buf).unwrap();
                Ok(u32::from_le_bytes(temp_buf) as u64)
            },
            RecIDSize::UINT64 => {
                let mut temp_buf = [0u8; 8];
                buf.read_exact(&mut temp_buf).unwrap();
                Ok(u64::from_le_bytes(temp_buf))
            }
        }
    }

    fn get_data_length(buf: &mut BufReader<File>, offset: u64) -> Result<u64, DynError> {
        // read DT/DV/DZ block's length
        buf.seek(SeekFrom::Start(offset+8))?;  // skip first 8 bytes
        let mut temp_buf = [0u8; 8];
        buf.read_exact(&mut temp_buf).unwrap();
        Ok(u64::from_le_bytes(temp_buf))
    }

    impl DataGroup {
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, DynError> {
            let dg_desc = get_block_desc_by_name("DG".to_string()).unwrap();
            let info = dg_desc.try_parse_buf(buf, offset)?;
            let rec_id_size = match info.get_data_value_first::<u8>("dg_rec_id_size") {
                Some(0) => RecIDSize::NORECID,
                Some(1) => RecIDSize::UINT8,
                Some(2) => RecIDSize::UINT16,
                Some(4) => RecIDSize::UINT32,
                Some(8) => RecIDSize::UINT64,
                _ => return Err("Unknown rec_id_size".into())
            };
            let comment = get_clean_text(buf, info.get_link_offset_normal("dg_md_comment").unwrap())
                                                            .unwrap_or("".to_string());
            let data = info.get_link_offset_normal("dg_data").unwrap();
            let cg_list: Vec<u64> = get_child_links(buf, 
                                    info.get_link_offset_normal("dg_cg_first").unwrap(), "CG").unwrap();
            let mut channel_groups: Vec<ChannelGroup> = Vec::new();
            cg_list.into_iter().for_each(|off| {
                let cg = ChannelGroup::new(buf, off).unwrap();
                channel_groups.push(cg)});
            let sorted = match channel_groups.len() {
                0 | 1 => true,
                _ => false
            };
            let mut rec_id_map: HashMap<u64, (u32, u64)> = HashMap::new();
            channel_groups.iter().for_each(|cg| {
                rec_id_map.insert(cg.get_record_id(), 
                (cg.get_sample_total_bytes(), cg.get_cycle_count()));
            });
            let mut offsets_map: HashMap<u64, Vec<u64>> = HashMap::new(); 
            let mut cycle_count_map: HashMap<u64, u64> = HashMap::new(); // used to temporarily store cycle count to verify if data corrupted or invalid
            let data_length = get_data_length(buf, data)?;
            buf.seek(SeekFrom::Current(8))?; // skip link_count
            let data_start = buf.stream_position()?;
            while buf.stream_position()?-data_start < data_length {
                let rec_id = read_rec_id(rec_id_size, buf)?;
                let offset = buf.stream_position()?;
                offsets_map.entry(rec_id)
                           .and_modify(|v| v.push(offset))
                           .or_insert(Vec::new());
                let bytes_to_skip = rec_id_map.get(&rec_id).unwrap().0;  // skip this record's data field
                buf.seek(SeekFrom::Current(bytes_to_skip as i64))?;
                cycle_count_map.entry(rec_id)
                               .and_modify(|v| {*v += 1})
                               .or_insert(1);
            }
            // check if cycle count is valid
            for (rec_id, cycle_count) in cycle_count_map.iter() {
                if rec_id_map.get(rec_id).unwrap().1 != *cycle_count {
                    return Err("Data corrupted: Invalid record cycle count.".into());
                }
            }
            Ok(Self { 
                rec_id_size, 
                comment, 
                data, 
                channel_groups,   
                sorted,
                rec_id_map,
                offsets_map
             })
        }

        pub fn create_map(&self) -> Result<HashMap<String, ChannelLink>, ()>{
            let mut hash_map: HashMap<String, ChannelLink> = HashMap::new();
            for cg in self.channel_groups.iter() {
                for channel in cg.get_channels().iter() {
                    hash_map.insert(channel.get_name().to_string(), ChannelLink(channel, cg, self));
                }
            }
            Ok(hash_map)
        }

        pub fn get_rec_id_size(&self) -> &RecIDSize {
            &self.rec_id_size
        }

        pub fn is_sorted(&self) -> bool {
            self.sorted
        }

        pub fn get_comment(&self) -> &str {
            &self.comment
        }

        pub fn get_cg_names(&self) -> Vec<String> {
            self.channel_groups.iter().map(|cg| cg.get_acq_name().to_string()).collect()
        }

        fn read_rec_id(&self, buf: &mut BufReader<File>, offset: u64) -> Result<u64, DynError> {
            // read record id to process ; Note this function will move buf's cursor
            buf.seek(SeekFrom::Start(offset)).unwrap();
            let rec_id_size = self.get_rec_id_size();
            match rec_id_size {
                RecIDSize::NORECID => Ok(0),
                RecIDSize::UINT8 => {
                    let mut temp_buf = [0u8; 1];
                    buf.read_exact(&mut temp_buf).unwrap();
                    Ok(temp_buf[0] as u64)},
                RecIDSize::UINT16 => {
                    let mut temp_buf = [0u8; 2];
                    buf.read_exact(&mut temp_buf).unwrap();
                    Ok(u16::from_le_bytes(temp_buf) as u64)
                },
                RecIDSize::UINT32 => {
                    let mut temp_buf = [0u8; 4];
                    buf.read_exact(&mut temp_buf).unwrap();
                    Ok(u32::from_le_bytes(temp_buf) as u64)
                },
                RecIDSize::UINT64 => {
                    let mut temp_buf = [0u8; 8];
                    buf.read_exact(&mut temp_buf).unwrap();
                    Ok(u64::from_le_bytes(temp_buf))
                }
            }
        }

        pub fn next_cg_chunk(&self, buf: &mut BufReader<File>, rec_id: u64) -> Result<(), DynError> {
            // assume buf's cursor is at the start of a record
            if let RecIDSize::NORECID = self.rec_id_size {
                let bytes_count = self.channel_groups[0].get_data_bytes();
                buf.seek(SeekFrom::Current(bytes_count as i64)).unwrap();
            }  
            else {
                let rec_id_read = self.read_rec_id(buf, self.data)?;
                if rec_id_read != rec_id {
                    
                }
            }
            Ok(())
        }
    }

    impl Display for DataGroup {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let cg_len = self.channel_groups.len();
            write!(f, "DataGroup Info:\nData offset at: 0x{:X}", self.data)?;
            write!(f, "\nComment: {}", self.comment)?;
            write!(f, "\nRecIDSize: {:?}", self.rec_id_size)?;
            write!(f, "\nChannelGroup number={}:", cg_len)?;
            for (ind, cg) in self.channel_groups.iter().enumerate() {
                write!(f, "\n\nChannelGroup [{}]", ind)?;
                for (ind, channel) in cg.get_channels().iter().enumerate() {
                    write!(f, "\n----Channel [{}]:", ind)?;
                    write!(f, "\n-----------{}", channel)?;
                }
            }
            write!(f, "\nEND DataGroup Info")
        }
    }
}