pub mod channelarray {
    use std::io::{BufReader, Cursor, Read};
    use std::fs::File;

    use crate::parser::get_block_desc_by_name;
    use crate::data_serde::DataValue;

    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct ChannelArray {
        ca_type: u8,
        ca_storage: u8,
        ca_ndim: u16,
        ca_flags: u32,
        ca_byte_offset_base: i32,
        ca_inval_bit_pos_base: u32,
        ca_dim_size: Vec<u64>,
    }

    impl ChannelArray {
        pub fn new(file: &mut BufReader<File>, offset: u64) -> Result<Self, Box<dyn std::error::Error>> {
            let block_desc = get_block_desc_by_name("CA".to_string()).unwrap();
            let block_info = block_desc.try_parse_buf(file, offset)?;
            let ca_type = block_info.get_data_value_first("ca_type").ok_or("Failed to get ca_type")?;
            let ca_storage = block_info.get_data_value_first("ca_storage").ok_or("Failed to get ca_storage")?;
            let ca_ndim:u16 = block_info.get_data_value_first("ca_ndim").ok_or("Failed to get ca_ndim")?;
            let ca_flags = block_info.get_data_value_first("ca_flags").ok_or("Failed to get ca_flags")?;
            let ca_byte_offset_base = block_info.get_data_value_first("ca_byte_offset_base").ok_or("Failed to get ca_byte_offset_base")?;
            let ca_inval_bit_pos_base = block_info.get_data_value_first("ca_inval_bit_pos_base").ok_or("Failed to get ca_inval_bit_pos_base")?;
            let mut ca_dim_size = Vec::new();
            if let DataValue::BYTE(bytes) = block_info.get_data_value("unparsed_data").ok_or("Invalid ca data without dim size")? {
                let mut cur = Cursor::new(bytes);
                for _ in 0..ca_ndim {
                    let mut eight_bytes = [0u8; 8];
                    cur.read_exact(&mut eight_bytes)?;
                    ca_dim_size.push(u64::from_le_bytes(eight_bytes));
                }
            }
            Ok( Self {
                ca_type,
                ca_storage,
                ca_ndim,
                ca_flags,
                ca_byte_offset_base,
                ca_inval_bit_pos_base,
                ca_dim_size,
            })
        }
    }
}