pub mod channelgroup {
    use std::io::Cursor;
    use crate::block::{BlockInfo, BlockDesc};
    use crate::parser::{get_clean_text, get_block_desc_by_name, get_child_links};
    use crate::components::si::sourceinfo::SourceInfo;
    use crate::components::cn::channel::Channel;

    #[derive(Debug, Clone)]
    pub struct ChannelGroup {
        acq_name: String,
        acq_source: SourceInfo,  
        comments: String,
        path_sep: String,
        record_id: u64,
        cycle_count: u64,
        data_bytes: u32,
        invalid_bytes: u32,
        channels: Vec<Channel>,
        master: Option<Channel>,
        cg_flags: u16,
        is_vlsd: bool,
        total_bytes: u64,   // for VLSD cg
    }

    impl ChannelGroup {
        
        pub fn new(buf:&mut Cursor<&[u8]>, offset: u64) -> Result<Self, Box<dyn std::error::Error>> {
            let cg_desc: &'static BlockDesc = get_block_desc_by_name("CG".to_string()).unwrap();
            let info: BlockInfo = cg_desc.try_parse_buf(buf, offset).unwrap();
            let acq_name: String = get_clean_text(buf, info.get_link_offset_normal("cg_tx_acq_name").unwrap())
                                    .unwrap_or("".to_owned());  // Nil is allowed
            let acq_source: SourceInfo = SourceInfo::new(buf, info.get_link_offset_normal("cg_si_acq_source").unwrap()).unwrap();
            let comments: String = get_clean_text(buf, info.get_link_offset_normal("cg_md_comment").unwrap())
                                    .unwrap_or("".to_owned());   // Nil is allowed
            let path_sep: String = match info.get_data_value_first::<u16>("cg_path_separator") {
                Some(0x2F) => "/".to_owned(),
                Some(0x5C) => "\\".to_owned(),
                _ => ".".to_owned(),
            };
            let record_id: u64 = info.get_data_value_first("cg_record_id")
                                    .ok_or("cg_data_bytes not found")?;
            let cycle_count: u64 = info.get_data_value_first("cg_cycle_count")
                                    .ok_or("cg_cycle_count not found")?;
            let data_bytes: u32 = info.get_data_value_first("cg_data_bytes")
                                    .ok_or("cg_data_bytes not found")?;
            let invalid_bytes: u32 = info.get_data_value_first("cg_inval_bytes")                                                                                                                                      
                                    .ok_or("cg_invalid_bytes not found")?;
            let cg_flags: u16 = info.get_data_value_first("cg_flags").ok_or("cg_flags not found")?;
            let mut channels: Vec<Channel> = Vec::new();
            let mut master: Option<Channel> = None;
            let mut is_vlsd: bool = false;
            let mut total_bytes: u64 = (data_bytes + invalid_bytes) as u64 * cycle_count ;
            if cg_flags & 0x0001 != 0 {
                is_vlsd = true;
                total_bytes = data_bytes as u64 | (invalid_bytes as u64) << 32;
            } else {
                let cn_link_list: Vec<u64> = get_child_links(buf, info.get_link_offset_normal("cg_cn_first").unwrap(), "CN").unwrap();
                cn_link_list.into_iter().for_each(|cn_link| {
                if let Ok(mut cn) = Channel::new(buf, cn_link) {
                    Self::new_channel_name(&mut cn, &acq_name);
                    if cn.is_master() {
                        master = Some(cn);
                    } else if cn.get_array().is_some() {
                        if let Ok(cn_array) = cn.generate_array_element_channel() {
                            channels.extend(cn_array);
                        }
                    } else if cn.is_composition() {
                        if let Ok(cn_composed) = cn.generate_composed_channels() {
                            channels.extend(cn_composed);
                        }
                    } else {
                        channels.push(cn);
                    }
                } else {
                    println!("Error reading channel at {}", cn_link);
                }
                });
            }
            Ok(Self {
                acq_name,
                acq_source,
                comments,
                path_sep,
                record_id,
                cycle_count,
                data_bytes,
                invalid_bytes,
                channels,
                master,
                cg_flags,
                is_vlsd,
                total_bytes,
                
            })
        }

        fn new_channel_name(cn: &mut Channel, acq_name: &String) {
            if cn.is_bus_event() && !acq_name.is_empty(){
                let mut name = String::from("");
                if !acq_name.is_empty() {
                    name.push_str(format!("{}.", acq_name).as_str());
                }
                //name.push_str(cn.get_name());   in case of bus event, channel name is not used
                if !cn.get_source().get_name().is_empty() {
                    name.push_str(format!("{}", cn.get_source().get_name()).as_str());
                } else if !cn.get_source().get_path().is_empty() {
                    name.push_str(format!("{}", cn.get_source().get_path()).as_str());
                } else {/* do nothing */}
                cn.set_name(name);   // change name for Bus Logging to avoid name duplication
            }
        }

        pub fn get_total_len(&self) -> u64 {
            self.total_bytes
        }

        pub fn is_vlsd(&self) -> bool {
            self.is_vlsd
        }

        pub fn get_cg_flags(&self) -> u16 {
            self.cg_flags
        }

        pub fn get_acq_name(&self) -> &str {
            &self.acq_name
        }

        pub fn get_acq_source(&self) -> &SourceInfo {
            &self.acq_source
        }

        pub fn get_comment(&self) -> &str {
            &self.comments
        }

        pub fn get_path_sep(&self) -> &str {
            &self.path_sep
        }

        pub fn get_record_id(&self) -> u64 {
            self.record_id
        }

        pub fn get_cycle_count(&self) -> u64 {
            self.cycle_count
        }

        pub fn get_data_bytes(&self) -> u32 {
            self.data_bytes
        }

        pub fn get_invalid_bytes(&self) -> u32 {
            self.invalid_bytes
        }

        pub fn get_sample_total_bytes(&self) -> u32 {
            self.data_bytes + self.invalid_bytes
        }

        pub fn get_channels(&self) -> &Vec<Channel> {
            &self.channels
        }

        pub fn get_channel_names(&self) -> Vec<String> {
            let names: Vec<String> = self.channels.iter()
                .map(|c| c.get_name().to_owned())
                .collect();
            names
        }

        pub fn get_master(&self) -> Option<&Channel> {
            self.master.as_ref()
        }

        pub fn nth_cn(&self, n: usize) -> Option<&Channel> {
            self.channels.get(n)
        }
    }
}


