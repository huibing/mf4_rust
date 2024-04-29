pub mod channelgroup {
    use std::io::BufReader;
    use std::fs::File;
    use crate::block::{BlockInfo, BlockDesc};
    use crate::parser::{get_clean_text, get_block_desc_by_name, get_child_links};
    use crate::components::si::sourceinfo::SourceInfo;
    use crate::components::cn::channel::Channel;

    #[derive(Debug)]
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
    }

    impl ChannelGroup {
        
        pub fn new(buf:&mut BufReader<File>, offset: u64) -> Result<Self, Box<dyn std::error::Error>> {
            let cg_desc: &'static BlockDesc = get_block_desc_by_name("CG".to_string()).unwrap();
            let info: BlockInfo = cg_desc.try_parse_buf(buf, offset).unwrap();
            let acq_name = get_clean_text(buf, info.get_link_offset_normal("cg_tx_acq_name").unwrap())
                                    .unwrap_or("".to_owned());  // Nil is allowed
            let acq_source = SourceInfo::new(buf, info.get_link_offset_normal("cg_si_acq_source").unwrap()).unwrap();
            let comments = get_clean_text(buf, info.get_link_offset_normal("cg_md_comment").unwrap())
                                    .unwrap_or("".to_owned());   // Nil is allowed
            let path_sep = match info.get_data_value_first::<u16>("cg_path_separator") {
                Some(0x2F) => "/".to_owned(),
                Some(0x5C) => "\\".to_owned(),
                _ => ".".to_owned(),
            };
            let record_id = info.get_data_value_first("cg_record_id")
                                    .ok_or("cg_data_bytes not found")?;
            let cycle_count = info.get_data_value_first("cg_cycle_count")
                                    .ok_or("cg_cycle_count not found")?;
            let data_bytes = info.get_data_value_first("cg_data_bytes")
                                    .ok_or("cg_data_bytes not found")?;
            let invalid_bytes: u32 = info.get_data_value_first("cg_inval_bytes")                                                                                                                                      
                                    .ok_or("cg_invalid_bytes not found")?;
            let mut channels = Vec::new();
            let cn_link_list = get_child_links(buf, info.get_link_offset_normal("cg_cn_first").unwrap(), "CN").unwrap();
            let mut master: Option<Channel> = None;
            cn_link_list.into_iter().for_each(|cn_link| {
                let cn = Channel::new(buf, cn_link).unwrap();
                if cn.is_master() {
                    master = Some(cn);
                } else if cn.get_array().is_some() {
                    if let Ok(cn_array) = cn.generate_array_element_channel() {
                        channels.extend(cn_array);
                    }
                } else {
                    channels.push(cn);
                }
            });
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
                master
            })
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
            self.channels.iter()
                .map(|c| c.get_name().to_owned())
                .collect()
        }

        pub fn get_master(&self) -> Option<&Channel> {
            self.master.as_ref()
        }

        pub fn nth_cn(&self, n: usize) -> Option<&Channel> {
            self.channels.get(n)
        }
    }
}


