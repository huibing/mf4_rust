/* Main user interface defined in DataGroup here. */

pub mod datagroup {
    use crate::components::cg::channelgroup::ChannelGroup;
    use crate::components::cn::channel::Channel;
    use crate::parser::{get_block_desc_by_name, get_clean_text, get_child_links};
    use crate::components::dx::dataxxx::{VirtualBuf, read_data_block};
    use crate::data_serde::{DataValue, FromLeBytes, FromBeBytes, UTF16String};
    use std::collections::HashMap;
    use std::io::BufReader;
    use std::fs::File;
    use std::fmt::Display;
    use half::f16;

    type DynError = Box<dyn std::error::Error>;
    #[derive(Debug, Clone, Copy)]
    pub enum RecIDSize {
        NORECID,
        UINT8,
        UINT16,
        UINT32,
        UINT64,
    }

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

        fn gen_value_vec<T>(&self, file: &mut BufReader<File>) -> Result<Vec<T>, DynError> 
        where T: FromLeBytes+FromBeBytes {
            let mut v: Vec<T> = Vec::new();
            for i in 0..self.get_channel_group().get_cycle_count() {
                let rec_data = self.get_data_group()    
                                            .get_cg_data(self.get_channel_group().get_record_id(), i, file)
                                            .ok_or("Invalid record id or cycle count.")?;
                v.push(self.get_channel().from_bytes::<T>(&rec_data).unwrap());
            }
            Ok(v)
        }

        pub fn yield_channel_data(&self, file: &mut BufReader<File>) -> Result<DataValue, DynError> {
            /* same as Channel.get_data_raw */
            let cn: &Channel = self.get_channel();
            let bits: u32 = cn.get_bit_size();
            match cn.get_data_type() {
                 0 | 1 => {
                    if bits <= 8 {
                        Ok(DataValue::UINT8(self.gen_value_vec::<u8>(file)?))
                    } else if bits>8 && bits <= 16 {
                        Ok(DataValue::UINT16(self.gen_value_vec::<u16>(file)?))
                    } else if bits>16 && bits <= 32 {
                        Ok(DataValue::UINT32(self.gen_value_vec::<u32>(file)?))
                    } else if bits>32 && bits <= 64 {
                        Ok(DataValue::UINT64(self.gen_value_vec::<u64>(file)?))
                    } else {
                        Err("Invalid bit size.".into())
                    }
                },
                2 | 3 => {
                    if bits <= 8 {
                        Ok(DataValue::INT8(self.gen_value_vec::<i8>(file)?))
                    } else if bits>8 && bits <= 16 {
                        Ok(DataValue::INT16(self.gen_value_vec::<i16>(file)?))
                    } else if bits>16 && bits <= 32 {
                        Ok(DataValue::INT32(self.gen_value_vec::<i32>(file)?))
                    } else if bits>32 && bits <= 64 {
                        Ok(DataValue::INT64(self.gen_value_vec::<i64>(file)?))
                    } else {
                        Err("Invalid bit size.".into())
                    }
                },
                4 | 5 => {
                    if bits == 16 {
                        Ok(DataValue::FLOAT16(self.gen_value_vec::<f16>(file)?))
                    } else if bits == 32 {
                        Ok(DataValue::SINGLE(self.gen_value_vec::<f32>(file)?))
                    } else if bits == 64 {
                        Ok(DataValue::REAL(self.gen_value_vec::<f64>(file)?))
                    } else {
                        Err("Invalid bit size.".into())
                    }
                },
                6 | 7 => {
                    Ok(DataValue::STRINGS(self.gen_value_vec::<String>(file)?))
                },
                8 | 9 => {
                    let s = self.gen_value_vec::<UTF16String>(file)?;
                    Ok(DataValue::STRINGS(s.into_iter().map(|s| s.inner).collect()))
                },
                _ => Err("Invalid data type.".into())
            }
        }

        pub fn get_master_channel_vec(&self, file: &mut BufReader<File>) -> Result<DataValue, DynError> {
            if self.get_channel().is_master() {
                // not possible if the channel link is build from current API
                Ok(self.yield_channel_data(file)?)
            } else {
                let cg: &ChannelGroup = self.get_channel_group();
                let dg: &DataGroup = self.get_data_group();
                let master_cn = cg.get_master()
                                            .ok_or::<DynError>("Cannot find master channel".into())?;
                Ok(master_cn.get_data_raw(file, dg, cg)?)
            }
        }
    }

    pub struct DataGroup{   
    // DataGroup is a container for ChannelGroups; this struct OWNS the ChannelGroups and its channels
        rec_id_size: RecIDSize,
        comment: String,
        data: u64,
        channel_groups: Vec<ChannelGroup>,
        sorted: bool,
        rec_id_map: HashMap<u64, (u32, u64)>, // record id -> (data bytes, cycle count)
        offsets_map: HashMap<u64, Vec<u64>>, // record id -> offset
        data_block: Box<dyn VirtualBuf>,  // one datagroup have one data_block
    }

    fn read_rec_id(rec_id_size: RecIDSize, buf: &Box<dyn VirtualBuf>, from:&mut BufReader<File>, v_offset: u64) -> Result<(u64, u8), DynError> {
        // read record id to process ; Note this function will move buf's cursor
        // u64: record id u8: bytes read
        match rec_id_size {
            RecIDSize::NORECID => Ok((0, 0u8)), // do nothing
            RecIDSize::UINT8 => {
                let mut temp_buf = [0u8; 1];
                buf.read_virtual_buf(from, v_offset,&mut temp_buf).unwrap();
                Ok((temp_buf[0] as u64, 1u8))},
            RecIDSize::UINT16 => {
                let mut temp_buf = [0u8; 2];
                buf.read_virtual_buf(from, v_offset,&mut temp_buf).unwrap();
                Ok((u16::from_le_bytes(temp_buf) as u64, 2u8))
            },
            RecIDSize::UINT32 => {
                let mut temp_buf = [0u8; 4];
                buf.read_virtual_buf(from, v_offset,&mut temp_buf).unwrap();
                Ok((u32::from_le_bytes(temp_buf) as u64, 4u8))
            },
            RecIDSize::UINT64 => {
                let mut temp_buf = [0u8; 8];
                buf.read_virtual_buf(from, v_offset,&mut temp_buf).unwrap();
                Ok((u64::from_le_bytes(temp_buf), 8u8))
            }
        }
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
            let cg_links: Vec<u64> = get_child_links(buf, 
                                    info.get_link_offset_normal("dg_cg_first").unwrap(), "CG").unwrap();
            let mut channel_groups: Vec<ChannelGroup> = Vec::new();
            cg_links.into_iter().for_each(|off| {
                let cg: ChannelGroup = ChannelGroup::new(buf, off).unwrap();
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
            let data_block = read_data_block(buf, data)?;
            let data_length = data_block.get_data_len(); // skip link_count
            let mut cur_off: u64 = 0; // virtual offset always start from 0
            while cur_off < data_length {
                let (rec_id, id_size) = read_rec_id(rec_id_size, &data_block, buf, cur_off)?;
                cur_off += id_size as u64;
                if !sorted{ /* for sorted dg, no offsets_map cache needed, it's easy to caculate record offset */
                    offsets_map.entry(rec_id)
                           .and_modify(|v| v.push(cur_off))
                           .or_insert(vec![cur_off]);
                }
                let bytes_to_skip = rec_id_map.get(&rec_id).unwrap().0;  // skip this record's data field
                cur_off += bytes_to_skip as u64;
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
                offsets_map,
                data_block
             })
        }

        pub fn create_map(&self) -> HashMap<String, ChannelLink>{
            let mut cn_link_map: HashMap<String, ChannelLink> = HashMap::new();
            for cg in self.channel_groups.iter() {
                for channel in cg.get_channels().iter() {
                    cn_link_map.insert(channel.get_name().to_string(), ChannelLink(channel, cg, self));
                }
            }
            cn_link_map
        }

        fn get_rec_id_offset(&self, rec_id: u64, cycle_index: u64) -> Option<u64> {
            if !self.sorted {
                self.offsets_map.get(&rec_id)
                                .and_then(|x| x.get(cycle_index as usize).and_then(|y| Some(y.clone())))
            } else {
                let bytes_num = self.rec_id_map.get(&rec_id)?.0;
                Some((cycle_index * bytes_num as u64) as u64)
            }
        }

        pub fn get_cg_data(&self, rec_id: u64, index: u64, file: &mut BufReader<File>) -> Option<Vec<u8>> {
            let virtual_offset = self.get_rec_id_offset(rec_id, index)?;
            let data_block = &self.data_block;
            let mut temp_buf = vec![0u8; self.rec_id_map.get(&rec_id)?.0 as usize];
            data_block.read_virtual_buf(file, virtual_offset, &mut temp_buf[..]).ok()?;
            Some(temp_buf)
        }

        pub fn get_all_channel_names(&self) -> Vec<String> {
            self.channel_groups.iter()
                               .flat_map(|cg| cg.get_channel_names()).collect()
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

        pub fn get_channle_groups(&self) -> &Vec<ChannelGroup> {
            &self.channel_groups
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