pub mod sourceinfo {
    use std::io::Cursor;
    use crate::block::BlockInfo;
    use crate::parser::{get_clean_text, get_block_desc_by_name};
    use std::fmt::Display;

    #[derive(Debug, Clone, Default)]
    pub enum SiType {
        #[default]
        OTHER,
        ECU,
        BUS,
        IO,
        TOOL,
        USER,
    }

    impl Display for SiType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::OTHER => write!(f, "Other"),
                Self::ECU => write!(f, "ECU"),
                Self::BUS => write!(f, "Bus"),
                Self::IO => write!(f, "IO"),
                Self::TOOL => write!(f, "Tool"),
                Self::USER => write!(f, "User"),
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    pub enum SiBusType {
        #[default]
        NONE,
        OTHER,
        CAN,
        LIN,
        MOST,
        FLEXRAY,
        KLINE,
        ETHERNET,
        USB
    }

    impl Display for SiBusType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::NONE => write!(f, "None"),
                Self::OTHER => write!(f, "Other"),
                Self::CAN => write!(f, "CAN"),
                Self::LIN => write!(f, "LIN"),
                Self::MOST => write!(f, "MOST"),
                Self::FLEXRAY => write!(f, "Flexray"),
                Self::KLINE => write!(f, "Kline"),
                Self::ETHERNET => write!(f, "Ethernet"),
                Self::USB => write!(f, "USB"),
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct SourceInfo {
        name: String,
        path: String,
        comment: String,
        si_type: SiType,
        bus_type: SiBusType,
        simulated: bool,
    }

    impl SourceInfo {
        pub fn new(buf: &mut Cursor<&[u8]>, offset: u64) -> Result<SourceInfo, Box<dyn std::error::Error>> {
            if offset == 0 {
                return Ok(Self::default()) // allows default
            }
            let si_desc = get_block_desc_by_name("SI".to_string()).unwrap();
            let info: BlockInfo = si_desc.try_parse_buf(buf, offset)?;
            let name_offset = info.get_link_offset_normal("si_tx_name")
                                                        .ok_or("Can not find source info name")?;
            let name = get_clean_text(buf, name_offset).unwrap_or("".to_string());
            let path_offset = info.get_link_offset_normal("si_tx_path")
                                                        .ok_or("Can not find source info path")?;
            let path = get_clean_text(buf, path_offset).unwrap_or("".to_string());
            let comment_offset = info.get_link_offset_normal("si_md_comment")
                                                    .ok_or("Can not find source info comment")?;
            let comment = get_clean_text(buf, comment_offset).unwrap_or("".to_string());
            let si_type = match info.get_data_value_first::<u8>("si_type") {
                Some(1) => SiType::ECU,
                Some(2) => SiType::BUS,
                Some(3) => SiType::IO,
                Some(4) => SiType::TOOL,
                Some(5) => SiType::USER,
                _ => SiType::OTHER,
            };
            let bus_type = match info.get_data_value_first::<u8>("si_bus_type") {
                Some(0) => SiBusType::NONE,
                Some(2) => SiBusType::CAN,
                Some(3) => SiBusType::LIN,
                Some(4) => SiBusType::MOST,
                Some(5) => SiBusType::FLEXRAY,
                Some(6) => SiBusType::KLINE,
                Some(7) => SiBusType::ETHERNET,
                Some(8) => SiBusType::USB,
                _ => SiBusType::OTHER,
            };
            let flags:u8 = info.get_data_value_first("si_flags").unwrap();
            let simulated = flags & 0x01 == 0x01;
            Ok(SourceInfo {
                name,
                path,
                comment,
                si_type,
                bus_type,
                simulated,
            })
        }


        pub fn get_name(&self) -> &str {
            &self.name
        }

        pub fn get_path(&self) -> &str {
            &self.path
        }

        pub fn get_comment(&self) -> &str {
            &self.comment
        }

        pub fn get_si_type(&self) -> &SiType {
            &self.si_type
        }

        pub fn get_bus_type(&self) -> &SiBusType {
            &self.bus_type
        }

        pub fn is_simulated(&self) -> bool {
            self.simulated  
        }
            
    }

    impl Display for SourceInfo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}.{}.{}", self.get_name(), self.get_si_type(), self.get_bus_type())
        }
    }
}
