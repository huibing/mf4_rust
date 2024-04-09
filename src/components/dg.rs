pub mod datagroup {
    use crate::components::cg::channelgroup::ChannelGroup;
    use crate::components::cn::channel::Channel;
    use crate::parser::{get_block_desc_by_name, get_clean_text, get_child_links};
    use std::collections::HashMap;
    use std::io::BufReader;
    use std::fs::File;
    use std::fmt::Display;

    #[derive(Debug)]
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
    }

    impl DataGroup {
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, Box<dyn std::error::Error>> {
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
            Ok(Self { 
                rec_id_size, 
                comment, 
                data, 
                channel_groups,    // move channel_groups into obj
                sorted,
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