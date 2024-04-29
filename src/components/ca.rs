pub mod channelarray {
    use std::io::{BufReader, Cursor, Read};
    use std::fs::File;

    use crate::parser::get_block_desc_by_name;
    use crate::data_serde::DataValue;

    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub struct ChannelArray {
        ca_type: u8,
        ca_storage: u8,
        ca_ndim: u16,
        ca_flags: u32,
        ca_byte_offset_base: i32,
        ca_inval_bit_pos_base: u32,
        ca_dim_size: Vec<u64>,
        f_vec: Vec<usize>,
        row_oriented: bool,
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
            let row_oriented = ca_flags & (0x01 << 6) != (0x01 << 6); // bit 6
            let mut f_vec: Vec<usize> = vec![ca_byte_offset_base as usize];
            if row_oriented {
                for i in 0..(ca_ndim-1) {
                    f_vec.push(f_vec.last().unwrap() * (ca_dim_size[(ca_ndim-1-i) as usize]) as usize);
                }
                f_vec.reverse();
            } else {
                for i in 0..(ca_ndim-1) {
                    f_vec.push(f_vec.last().unwrap() * (ca_dim_size[i as usize]) as usize);
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
                f_vec,
                row_oriented,
            })
        }

        pub fn generate_array_names(&self, channel_name: &str) -> Vec<String> {
            let mut array_names = Vec::new();
            fn dfs_gen(prefix: &mut String, array: &ChannelArray, n_dim: u64, res: &mut Vec<String>) {
                if n_dim == (array.ca_ndim - 1) as u64 {
                    for i in 0..array.ca_dim_size[n_dim as usize] {
                        let mut new_str = prefix.clone();
                        new_str.push_str(format!("[{}]", i).as_str());
                        res.push(new_str);
                    }
                } else {
                    for i in 0..array.ca_dim_size[n_dim as usize] {
                        let mut new_str = prefix.clone();
                        new_str.push_str(format!("[{}]", i).as_str());
                        dfs_gen(&mut new_str, array, n_dim + 1, res);
                    }
                }
            }
            let mut s = format!("{}", channel_name);
            dfs_gen(&mut s, self, 0, &mut array_names);
            array_names
        }

        pub fn generate_array_indexs(&self) -> Vec<Vec<usize>> { // seperate function for simplicity
            let mut array_indexs = Vec::new();
            fn dfs_gen(index: &mut Vec<usize>, array: &ChannelArray, n_dim: u64, res: &mut Vec<Vec<usize>>) {
                if n_dim == (array.ca_ndim - 1) as u64 {
                    for i in 0..array.ca_dim_size[n_dim as usize] {
                        let mut new_index = index.clone();
                        new_index.push(i as usize);
                        res.push(new_index);
                    }
                } else {
                    for i in 0..array.ca_dim_size[n_dim as usize] {
                        let mut new_index = index.clone();
                        new_index.push(i as usize);
                        dfs_gen(&mut new_index, array, n_dim + 1, res);
                    }
                }
            }
            let mut index = vec![];
            dfs_gen(&mut index, self, 0, &mut array_indexs);
            array_indexs
        }

        pub fn calculate_byte_offset(&self, index: &Vec<usize>) -> Result<u32, Box<dyn std::error::Error>> {
            if index.len() != self.ca_ndim as usize {
                return Err("Invalid index array length for CA".into());
            } else {
                Ok(index.iter().zip(self.f_vec.iter()).fold(0, |acc, y| acc + (y.1 * y.0) as u32))
            }
        }

        pub fn get_elements_num(&self) -> usize {
            self.ca_dim_size.iter().fold(1, |acc, y| acc * (*y as usize))
        }
    }
}