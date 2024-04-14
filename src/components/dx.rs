/* DL DV DT SD RD blocks*/

pub mod dataxxx {
    use std::io::{Read, Seek, BufReader, SeekFrom};
    use std::fs::File;

    type DynError = Box<dyn std::error::Error>;

    /* This trait should be implemented to DT SD and RD DL blocks
       This trait is used to read physically incontinuous data block linked by DL block*/
    pub trait VirtualBuf{
        fn read_virtual_buf(&self, from: &mut BufReader<File>, virtual_offset:u64, buf: &mut [u8]) 
            -> Result<(), DynError>;
        
        fn get_data_len(&self) -> u64;
    }
    #[derive(Debug)]
    pub struct DT{
        data_len: u64,
        data_offset: u64,   // absolute offset in file
    }

    impl DT{
        /* This should also works for SD and RD blocks; they have samilar data structure  */
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, DynError>{
            let mut data_buf = [0u8; 4];
            buf.seek(SeekFrom::Start(offset))?;
            buf.read_exact(&mut data_buf)?;
            if String::from_utf8(data_buf.to_vec()).unwrap() != "##DT"{
                return Err("Invalid DT block".into());
            } else {
                buf.seek(SeekFrom::Current(4))?; // skip 4 reserved bytes
                let mut buffer = [0u8; 8];
                buf.read_exact(&mut buffer)?;
                buf.seek(SeekFrom::Current(8))?; // skip 8 bytes that are link len
                Ok(DT{
                    data_len: u64::from_le_bytes(buffer) - 24,  // without header
                    data_offset: buf.stream_position()?,
                })
            }
        }
    }

    impl VirtualBuf for DT{
        fn read_virtual_buf(&self, from: &mut BufReader<File>, virtual_offset:u64, buf: &mut [u8]) -> Result<(), DynError> {
            let file_offset = self.data_offset + virtual_offset;
            from.seek(SeekFrom::Start(file_offset))?;
            let left_bytes_num = self.data_len - virtual_offset;
            let data_to_read = buf.len();
            if data_to_read > left_bytes_num as usize {
                Err("Not enough bytes in the block".into())
            } else {
                from.read_exact(buf)?;
                Ok(())
            }
        }

        fn get_data_len(&self) -> u64 {
            self.data_len
        }
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    struct DLBlock {
        dl_dl_next: u64,
        dl_data: Vec<u64>,
        dl_flags: u8,
        dl_count: u32,
        dl_equal_length: Option<u64>,
        dl_offset: Option<Vec<u64>>,
        dl_time_values: Option<Vec<u64>>,
        dl_angle_values: Option<Vec<u64>>,
        dl_distance_values: Option<Vec<u64>>,
    }

    #[derive(Debug)]
    pub struct DataLink {// T is the type of data block
        /* this struct will collect DL block links into one bulk */
        total_len: u64,
        num_of_blocks: u64,
        start_offsets_in_file: Vec<u64>,  // file abolute offset
        virtual_offsets: Vec<u64>,
        data_blocks: Vec<DT>,
    }

    fn read_dl_block(buf: &mut BufReader<File>, offset: u64) -> Result<DLBlock, DynError> {
        /* helper function to read DL block info to construct DataLink */
        buf.seek(SeekFrom::Start(offset))?;
        let mut buffer = [0u8; 4];
        buf.read_exact(&mut buffer)?;

        if String::from_utf8(buffer.to_vec()).unwrap() != "##DL"{
            return Err("Invalid DL block".into());
        } else {
            buf.seek(SeekFrom::Current(4))?; // skip 4 reserved bytes
            let mut eight_bytes = [0u8; 8];
            buf.read_exact(&mut eight_bytes)?;
            //let dl_len = u64::from_le_bytes(eight_bytes) - 24;  // head is not included
            buf.read_exact(&mut eight_bytes)?;
            let link_len = u64::from_le_bytes(eight_bytes);
            buf.read_exact(&mut eight_bytes)?;
            let dl_dl_next = u64::from_le_bytes(eight_bytes);
            let mut dl_data = Vec::new();
            for _ in 0..link_len-1 {
                buf.read_exact(&mut eight_bytes)?;
                dl_data.push(u64::from_le_bytes(eight_bytes));
            };
            buf.read_exact(&mut eight_bytes)?;
            let dl_flags = eight_bytes[0];
            let dl_count = u32::from_le_bytes(eight_bytes[4..].try_into().unwrap());
            let dl_equal_length = if dl_flags & 0x01 == 0x01 {
                buf.read_exact(&mut eight_bytes)?;
                Some(u64::from_le_bytes(eight_bytes))
            } else {
                None
            };
            let dl_offset = if dl_flags & 0x01 == 0x00 {
                let mut v:Vec<u64> = Vec::new();
                (0..dl_count).for_each(|_| {
                    buf.read_exact(&mut eight_bytes).unwrap();
                    v.push(u64::from_le_bytes(eight_bytes));
                });
                Some(v)
            } else {
                None
            };
            let dl_time_values = if dl_flags & 0x02 == 0x02 {
                let mut v = Vec::new();
                (0..dl_count).for_each(|_| {
                    buf.read_exact(&mut eight_bytes).unwrap();
                    v.push(u64::from_le_bytes(eight_bytes));
                });
                Some(v)
            } else {
                None
            };
            let dl_angle_values = if dl_flags & 0x04 == 0x04 {
                let mut v = Vec::new();
                (0..dl_count).for_each(|_| {
                    buf.read_exact(&mut eight_bytes).unwrap();
                    v.push(u64::from_le_bytes(eight_bytes));
                });
                Some(v)
            } else {
                None
            };
            let dl_distance_values = if dl_flags & 0x08 == 0x08 {
                let mut v = Vec::new();
                (0..dl_count).for_each(|_| {
                    buf.read_exact(&mut eight_bytes).unwrap();
                    v.push(u64::from_le_bytes(eight_bytes));
                });
                Some(v)
            } else {
                None
            };
            Ok(DLBlock{
                dl_dl_next,
                dl_data,
                dl_flags,
                dl_count,
                dl_equal_length,
                dl_offset,
                dl_time_values,
                dl_angle_values,
                dl_distance_values,
            })
        }
    }

    impl DataLink {
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, DynError>{
            let mut dl_blocks = Vec::new();
            let mut cur_off = offset;
            loop {
                let dl_block = read_dl_block(buf, cur_off)?;
                if dl_block.dl_dl_next == 0 {
                    dl_blocks.push(dl_block);
                    break;
                } else {
                    cur_off = dl_block.dl_dl_next;
                    dl_blocks.push(dl_block);
                }
            }
            let num_of_blocks = dl_blocks.iter()
                        .fold(0, |acc, x| acc + x.dl_count as u64);
            let start_offsets_in_file = dl_blocks.iter().flat_map(|x| {
                x.dl_data.iter().map(|y| {
                    *y
                })
            }).collect();
            let data_blocks: Vec<DT> = dl_blocks.iter().flat_map(|x| {
                x.dl_data.iter().map(|y| {
                    DT::new(buf, *y).unwrap()
                }).collect::<Vec<DT>>()
            }).collect();
            let total_len:u64 = data_blocks.iter()
                .fold(0, |acc, x| acc + x.data_len);
            let virtual_offsets = {
                let mut v = vec![0u64];  // start from 0
                let mut cur_offset: u64 = 0;
                data_blocks.iter().for_each(|x| {
                    cur_offset += x.data_len;
                    v.push(cur_offset);
                });
                v.pop();  // pop out the last item which is the total length of serval DT blocks
                v
            };
            /* verifiy offsets if not equal length */
            if dl_blocks[0].dl_flags & 0x01 == 0x00 {
                // every dl block should be not equal length then
                let equal_len = dl_blocks.iter().all(|x| x.dl_flags & 0x01 == 0x00);
                if !equal_len {
                    return Err("Not all dl blocks have the same equal length flag status.".into());
                } else {
                    let offs:Vec<u64> = dl_blocks.iter().flat_map(|x| {
                        let v = x.dl_offset.as_ref().unwrap();
                        v.iter().map(|y| {
                            y.clone()
                        })
                    }).collect();
                    for (left, right) in offs.iter().zip(virtual_offsets.iter()) {
                        if left != right {
                            return Err("Offset not right for DL links.".into());
                        }
                    }
                }
            }
            Ok(DataLink{
                total_len,
                num_of_blocks,
                start_offsets_in_file,
                virtual_offsets,
                data_blocks,
            })
        }

        pub fn get_num_of_blocks(&self) -> u64 {
            self.num_of_blocks
        }

        pub fn get_total_len(&self) -> u64 {
            self.total_len
        }

        pub fn get_start_offsets_in_file(&self) -> &Vec<u64> {
            &self.start_offsets_in_file
        }

        pub fn get_virtual_offsets(&self) -> &Vec<u64> {
            &self.virtual_offsets
        }
    }

    impl VirtualBuf for DataLink {
        fn read_virtual_buf(&self, from: &mut BufReader<File>, virtual_offset:u64, buf: &mut [u8]) 
        -> Result<(), DynError> {
            let end_index = virtual_offset + buf.len() as u64;
            if end_index > self.total_len {
                return Err("Virtual offset out of range.".into());
            } else {
                let start_block_id: usize = {
                    // there is chance that start or end index exceeds the max of virtual offsets
                    self.virtual_offsets.iter()
                            .position(|x| *x > virtual_offset)
                            .unwrap_or(self.virtual_offsets.len()) - 1
                };
                let end_block_id: usize = {
                    self.virtual_offsets.iter()
                            .position(|x| *x > end_index)
                            .unwrap_or(self.virtual_offsets.len()) - 1
                };
                if start_block_id == end_block_id {
                    /* data in one DT block */
                    let data_block = &self.data_blocks[start_block_id];
                    let data_start_virtual_offset = self.virtual_offsets[start_block_id];
                    data_block.read_virtual_buf(from,
                             virtual_offset-data_start_virtual_offset, buf)?;
                } else {
                    /* data span across two or more physical DT block */
                    let blocks = self.data_blocks[start_block_id..=end_block_id]
                                        .iter().zip(&self.virtual_offsets[start_block_id..=end_block_id]);
                    let mut cur_offset:u64 = virtual_offset;
                    for (block, block_start_v_offset) in blocks {
                        let relative_offset: u64 = cur_offset - block_start_v_offset;
                        let bytes_to_read: u64 = (block.data_len - relative_offset).min(end_index-cur_offset); // last block will use end - cur instead
                        block.read_virtual_buf(from, 
                                relative_offset, 
                                &mut buf[(cur_offset-virtual_offset) as usize..(bytes_to_read+cur_offset-virtual_offset) as usize])?;
                        cur_offset += bytes_to_read;  // update cursor offset for next iteration
                    }
                }   
                Ok(())
            }
        }

        fn get_data_len(&self) -> u64 {
            self.total_len
        }
    }

    pub fn read_data_block(buf: &mut BufReader<File>, offset: u64) -> Result<Box<dyn VirtualBuf>, DynError> {
        buf.seek(SeekFrom::Start(offset))?;
        let mut four_bytes_id = [0u8; 4];
        buf.read_exact(&mut four_bytes_id)?;
        let id = String::from_utf8(four_bytes_id.to_vec()).unwrap();
        match id.as_str() {
            "##DT" => Ok(Box::new(DT::new(buf, offset)?)),
            "##DL" => Ok(Box::new(DataLink::new(buf, offset)?)),
            _ => Err("Unknown data block id.".into()),
        }
    }
}