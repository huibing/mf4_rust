pub mod channel {
    use std::io::{Cursor, BufReader};
    use std::fs::File;
    use std::fmt::Display;
    use half::f16;
    use indexmap::IndexMap;

    use crate::block::BlockDesc;
    use crate::components::dg::datagroup::DataGroup;
    use crate::parser::{get_block_desc_by_name, get_child_links, get_clean_text, peek_block_type};
    use crate::components::cc::conversion::Conversion;
    use crate::components::si::sourceinfo::SourceInfo;
    use crate::components::cg::channelgroup::ChannelGroup;
    use crate::data_serde::{bytes_and_bits, right_shift_bytes, DataValue, FromBeBytes, FromLeBytes, UTF16String};
    use crate::components::dx::dataxxx::{read_data_block, VirtualBuf};
    use crate::components::ca::channelarray::ChannelArray;
    
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
        bytes_num: u32,
        master: bool,
        cn_data: u64,
        sub_channels: Option<Vec<Channel>>,   // for composed signals; also for MLSD channel
        array: Option<ChannelArray>,
        cn_flags: u32,
        cn_compositon: u64,
    }

    impl Channel {
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, DynError> {
            let desc: &BlockDesc = get_block_desc_by_name("CN".to_string()).expect("CN block not found");
            let info: crate::block::BlockInfo = desc.try_parse_buf(buf, offset).unwrap();
            let name: String = get_clean_text(buf, info.get_link_offset_normal("cn_tx_name").unwrap())
                               .unwrap_or("".to_string());
            let source: SourceInfo = SourceInfo::new(buf, info.get_link_offset_normal("cn_si_source").unwrap())?;
            let conversion: Conversion = Conversion::new(buf, info.get_link_offset_normal("cn_cc_conversion").unwrap())?;
            let unit: String = get_clean_text(buf, info.get_link_offset_normal("cn_md_unit").unwrap())
                                .unwrap_or("".to_string());
            let comment: String = get_clean_text(buf, info.get_link_offset_normal("cn_md_comment").unwrap())
                                .unwrap_or("".to_string());
            let cn_type: u8 = info.get_data_value_first("cn_type").ok_or("cn_type not found")?;
            let master: bool = match cn_type {
                0 | 1 | 5 | 6 => false,  // 5 :: MLSD 
                2 | 3 => true,
                _ => return Err(format!("cn_type {} not supportted yet.", cn_type).into()),
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
            let cn_data: u64 = info.get_link_offset_normal("cn_data").unwrap_or(0);
            let cn_compositon = info.get_link_offset_normal("cn_composition").unwrap_or(0);
            let (mut sub_channels, array) = if let Ok(block_type) = peek_block_type(buf, cn_compositon) {
                match block_type.as_str() {
                    "CN" => {
                        if data_type == 10 {
                            let mut channels: Vec<Channel> = Vec::new();
                            let links: Vec<u64> = get_child_links(buf, cn_compositon, "CN")?;
                            links.iter().for_each(|l: &u64| {
                                if let Ok(cn) = Channel::new(buf, *l) {   // this could be recursive
                                    channels.push(cn);
                                }
                            });
                            (Some(channels), None)
                        } else {
                            (None, None)
                        }
                    },
                    "CA" => {
                        if let Ok(ca) = ChannelArray::new(buf, cn_compositon) {   // this could be recursive
                            (None, Some(ca))
                        } else { (None, None) }    // this could fail because of CG or DG template
                    },
                    _ => (None, None)
                }
            } else { (None, None) };
            let cn_flags: u32 = info.get_data_value_first::<u32>("cn_flags").ok_or("CN flags not found")?;
            if cn_type == 5u8 {
                if cn_data != 0x00u64 {
                    if let Ok(ch) = Channel::new(buf, cn_data) {
                        sub_channels = Some(vec![ch]);
                    } else {
                        return Err("CN data should be a channel for MLSD channel.".into())
                    }
                } else {
                    return Err("CN data should not be empty for MLSD channel.".into())
                }
            }
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
                master,
                bytes_num,
                cn_data,
                sub_channels,
                array,
                cn_flags,
                cn_compositon
            })
        }

        pub fn get_name(&self) -> &str {
            &self.name
        }

        pub fn get_sync_type(&self) -> &SyncType {
            &self.sync_type
        }

        pub fn get_cn_flags(&self) -> u32 {
            self.cn_flags
        }

        pub fn get_cn_type(&self) -> &u8 {
            &self.cn_type
        }

        pub fn get_array(&self) -> Option<&ChannelArray> {
            self.array.as_ref()
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

        pub fn is_master(&self) -> bool {
            self.master
        }

        pub fn get_data_raw(&self, file: &mut BufReader<File>, dg: &DataGroup, cg: &ChannelGroup) -> Result<DataValue, DynError> {
            let bits: u32 = self.get_bit_size();
            if self.get_cn_type() == &1 { 
                // special case for VLSD ; record bytes are only offset of SD blocks 
                return Ok(DataValue::UINT64(self.gen_value_vec::<u64>(file, dg, cg)?))
            } else if self.get_cn_type() == &6 || self.get_cn_type() == &3{ // virtual data channel
                if self.get_data_type() == 0 {
                    return Ok(DataValue::UINT64((0..cg.get_cycle_count()).collect()))
                } else {
                    return Err("Virtual data channel only support little-edian unsigned interger data type".into())
                }
            }
            match self.get_data_type() {
                0 | 1 => {
                    if bits <= 8 {
                        Ok(DataValue::UINT8(self.gen_value_vec(file, dg, cg)?))
                    } else if bits>8 && bits <= 16 {
                        Ok(DataValue::UINT16(self.gen_value_vec(file, dg, cg)?))
                    } else if bits>16 && bits <= 32 {
                        Ok(DataValue::UINT32(self.gen_value_vec::<u32>(file, dg, cg)?))
                    } else if bits>32 && bits <= 64 {
                        Ok(DataValue::UINT64(self.gen_value_vec::<u64>(file, dg, cg)?))
                    } else {
                        Err("Invalid bit size.".into())
                    }
                },
                2 | 3 => {
                    if bits <= 8 {
                        Ok(DataValue::INT8(self.gen_value_vec::<i8>(file, dg, cg)?))
                    } else if bits>8 && bits <= 16 {
                        Ok(DataValue::INT16(self.gen_value_vec::<i16>(file, dg, cg)?))
                    } else if bits>16 && bits <= 32 {
                        Ok(DataValue::INT32(self.gen_value_vec::<i32>(file, dg, cg)?))
                    } else if bits>32 && bits <= 64 {
                        Ok(DataValue::INT64(self.gen_value_vec::<i64>(file, dg, cg)?))
                    } else {
                        Err("Invalid bit size.".into())
                    }
                },
                4 | 5 => {
                    if bits == 16 {
                        Ok(DataValue::FLOAT16(self.gen_value_vec::<f16>(file, dg, cg)?))
                    } else if bits == 32 {
                        Ok(DataValue::SINGLE(self.gen_value_vec::<f32>(file, dg, cg)?))
                    } else if bits == 64 {
                        Ok(DataValue::REAL(self.gen_value_vec::<f64>(file, dg, cg)?))
                    } else {
                        Err("Invalid bit size.".into())
                    }
                },
                6 | 7 => {
                    Ok(DataValue::STRINGS(self.gen_value_vec::<String>(file, dg, cg)?))
                },
                8 | 9 => {
                    let s: Vec<UTF16String> = self.gen_value_vec::<UTF16String>(file, dg, cg)?;
                    Ok(DataValue::STRINGS(s.into_iter().map(|s| s.inner).collect()))
                },
                _ => Err("Invalid data type.".into())
            }
        }

        fn get_byte_array(&self, file: &mut BufReader<File>, dg: &DataGroup, cg: &ChannelGroup) -> Result<Vec<Vec<u8>>, DynError> {
            let mut bytes_array_vec: Vec<Vec<u8>> = Vec::new();
            for i in 0..cg.get_cycle_count() {
                let rec_data: Vec<u8> = dg.get_cg_data(cg.get_record_id(), i, file)
                                          .ok_or("error during get cg record bytes")?;
                let raw_data: Vec<u8> = rec_data[self.byte_offset as usize
                                            ..(self.byte_offset+self.get_bytes_num()) as usize].to_vec();
                let bytes_array: Vec<u8> = if self.bit_offset != 0 {
                    let mut arr: Vec<u8> = right_shift_bytes(&raw_data, self.bit_offset)?;
                    bytes_and_bits(&mut arr, self.bit_count);
                    arr
                } else { raw_data };
                bytes_array_vec.push(bytes_array);
            }
            Ok(bytes_array_vec)
        }

        pub fn get_data(&self, file: &mut BufReader<File>, dg: &DataGroup, cg: &ChannelGroup) -> Result<DataValue, DynError> {
            if self.data_type == 10 && self.sub_channels.is_some() && self.cn_compositon != 0{   // for compact structure
                let mut value_map: IndexMap<String, DataValue> = IndexMap::new();
                for cn in self.sub_channels.as_ref().unwrap() {  // Currently, this structure decompose is done while get_data runtime; however this process should be done in channel's consturction time
                    cn.get_data(file, dg, cg)  // this could be recursive
                      .and_then(|data| {
                        value_map.insert(cn.get_name().to_string(), data);
                        Ok(())
                      }).unwrap_or(());    
                }
                return Ok(DataValue::STRUCT(value_map))
            } else if self.data_type == 10 { // for pure BYTEARRAY
                return Ok(DataValue::BYTEARRAY(self.get_byte_array(file, dg, cg)?))
            }
            let data_raw: DataValue = self.get_data_raw(file, dg, cg)?;
            if self.get_cn_type() == &1 {                                 // for VLSD with SD blocks; not suitable for VLSD with channel groups
                let offsets: Vec<u64> = data_raw.try_into()?;
                return self.parse_sd_data(file, &offsets)
            }
            if data_raw.is_num() {
                let float_data: Vec<f64> = data_raw.try_into()?;
                if self.get_conversion().get_cc_type().is_num() {
                    Ok(DataValue::REAL(float_data.into_iter().map(|f| self.get_conversion().transform_value(f)).collect()))
                } else {
                    Ok(DataValue::STRINGS(float_data.into_iter().map(|f| self.get_conversion().convert_to_text(file, f).unwrap()).collect()))  // todo: remove unwrap handle errors
                }
            } else {
                Ok(data_raw)
            }
        }

        fn gen_value_vec<T>(&self, file: &mut BufReader<File>, dg: &DataGroup, cg: &ChannelGroup) -> Result<Vec<T>, DynError> 
        where T: FromBeBytes + FromLeBytes {  /* function used to read record bytes into channel value*/
            let mut values: Vec<T> = Vec::new();
            for i in 0..cg.get_cycle_count() {
                let rec_data = dg.get_cg_data(cg.get_record_id(), i, file)
                                        .ok_or("Invalid record id or cycle count.")?;
                values.push(self.from_bytes::<T>(&rec_data)?);
            }
            Ok(values)
        }

        pub fn from_bytes<T>(&self, rec_bytes: &Vec<u8>) -> Result<T, DynError> 
        where T: FromBeBytes + FromLeBytes
        {
            let raw_data: Vec<u8> = rec_bytes[self.byte_offset as usize..
                                (self.byte_offset + self.get_bytes_num()) as usize].to_vec();
            let cn_data: Vec<u8> = if self.bit_offset != 0 {
                let mut new_bytes: Vec<u8> = right_shift_bytes(&raw_data, self.bit_offset)?;
                bytes_and_bits(&mut new_bytes, self.bit_count);
                new_bytes
            } else {
                raw_data
            };
            let mut data_buf: Cursor<Vec<u8>> = Cursor::new(cn_data);
            match self.data_type {
                // only distinguish little-edian and big-endian here. Concrete data types are handled in the up
                // level functions.
                0|2|4|6|7|8 => Ok(T::from_le_bytes(&mut data_buf)),
                1|3|5|9 => Ok(T::from_be_bytes(&mut data_buf)),
                _ => Err("data type not supportted.".into()),
            }
        }

        fn get_bytes_num(&self) -> u32 {
            self.bytes_num
        }

        fn parse_sd_data(&self, file: &mut BufReader<File>, offsets: &Vec<u64>) -> Result<DataValue, DynError> {
            let data_blocks: Box<dyn VirtualBuf> = read_data_block(file, self.cn_data)?;
            let mut sd_data: Vec<String> = Vec::new();  // todo: is there any other possible data types?
            for offset in offsets.iter() {
                let mut four_bytes: [u8; 4] = [0u8; 4];
                data_blocks.read_virtual_buf(file, *offset, &mut four_bytes)?;
                let length: u32 = u32::from_le_bytes(four_bytes);
                let mut data_bytes: Vec<u8> = vec![0u8; length as usize];
                data_blocks.read_virtual_buf(file, *offset + 4, &mut data_bytes)?;
                match self.get_data_type() {
                    6 | 7 => {
                        let raw: String = String::from_utf8(data_bytes)?;
                        sd_data.push(raw.trim_end_matches('\0').to_string());
                    },
                    8 => {
                        let mut data: Cursor<Vec<u8>> = Cursor::new(data_bytes);
                        let u16str: UTF16String = UTF16String::from_le_bytes(&mut data);
                        sd_data.push(u16str.inner.trim_end_matches('\0').to_string());
                    },
                    9 => {
                        let mut data: Cursor<Vec<u8>> = Cursor::new(data_bytes);
                        let u16str: UTF16String = UTF16String::from_be_bytes(&mut data);
                        sd_data.push(u16str.inner.trim_end_matches('\0').to_string());
                    },
                    num => {
                        return Err(format!("Can not parse sd data with this type {}", num).into())
                    }
                }
            }
            Ok(DataValue::STRINGS(sd_data))
        }

        fn set_name(&mut self, name: String) {
            self.name = name;
        }

        fn change_byte_offset(&mut self, offset: u32) {
            self.byte_offset = offset;
        }

        pub fn generate_array_element_channel(self) -> Result<Vec<Self>, DynError> {
            // this function will consume self
            if self.get_array().is_none() {
                Err("Not an array element channel.".into())
            } else {
                let mut channels: Vec<Self> = Vec::new();  // element 0
                let ca: &ChannelArray = self.get_array().unwrap();
                let indexes: Vec<Vec<usize>> = ca.generate_array_indexs();
                let names: Vec<String> = ca.generate_array_names(self.get_name());
                for (index, name) in indexes.iter().zip(names.iter()) {
                    let new_bytes_offset: u32 = ca.calculate_byte_offset(index)?;
                    let mut new_channel: Channel = self.clone();
                    new_channel.change_byte_offset(new_bytes_offset);
                    new_channel.set_name(name.to_string());
                    channels.push(new_channel);
                }
                Ok(channels)
            }
        }

    }

    impl Display for Channel {
        // for debug purpose
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Channel: {}", self.name)?;
            write!(f, "\n-----------Source: {}", self.source)?;
            write!(f, "\n-----------ChannelType: {}", self.cn_type)?;
            write!(f, "\n-----------SyncType: {:?}", self.sync_type)?;
            write!(f, "\n-----------Conversion: {}: {:?}", self.conversion.get_cc_name(), self.conversion.get_cc_type())?;
            write!(f, "\n-----------Unit: {}", self.unit)?;
            write!(f, "\n-----------DataType: {}", self.data_type)?;
            write!(f, "\n-----------ChannelType: {}", self.cn_type)?;
            write!(f, "\n-----------BitSize: {}", self.get_bit_size())?;
            write!(f, "\nEND Channel ")
        }
    }
}

