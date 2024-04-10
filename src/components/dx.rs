/* DL DV DT  blocks*/

pub mod dx {
    use std::io::{Read, Seek, BufReader, SeekFrom};
    use std::fs::File;

    type DynError = Box<dyn std::error::Error>;
    #[derive(Debug)]
    pub struct DT{
        data_len: u64,
        start_offset: u64,
    }

    impl DT{
        
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
                    data_len: u64::from_le_bytes(buffer) - 24,
                    start_offset: buf.stream_position()?,
                })
            }
        }
    }
}