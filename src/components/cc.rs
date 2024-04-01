pub mod conversion {
    use std::fs::File;
    use std::io::BufReader;
    use crate::block::{BlockInfo, DataValue};
    use crate::parser::{get_tx_data, get_text};

    #[derive(Debug)]
    pub struct Conversion 
    {
        name: String,
        unit: String,
        comment: String,
        inverse_ref: u64, // TODO: implement inverse conversion
        cc_type: CcType,
    } 
    
    #[derive(Debug)]
    pub enum CcType {
        OneToOne,
        Linear((f64, f64)),
        Rational([f64; 6]),
        Algebraic(String),
        TableInt(Vec<f64>),  // table with interpolation
        Table(Vec<f64>), // table without interpolation
        ValueRange(Vec<f64>),
        ValueText((Vec<f64>, Vec<u64>)),   //first from cc_val, second from cc_ref
        ValueRangeText((Vec<f64>, Vec<u64>)), 
        Text2Value((Vec<u64>, Vec<f64>)),
        Text2Text(Vec<u64>),
        BitfieldText((Vec<u64>, Vec<u64>)),
        NotImplemented
    }

    fn to_f64(v: u64) -> f64 {
        // change to raw bytes
        let bytes = v.to_le_bytes();
        f64::from_le_bytes(bytes)  // return f64 value
    }

    impl Conversion {
        pub fn new(block_info: &BlockInfo, buf: &mut BufReader<File>) -> Result<Self, Box<dyn std::error::Error>> {
            let name: String = get_tx_data(buf, block_info.get_link_offset_normal("cc_tx_name").unwrap()).unwrap();
            let unit: String = get_text(buf, block_info.get_link_offset_normal("cc_unit").unwrap()).unwrap();
            let comment: String = get_text(buf, block_info.get_link_offset_normal("cc_comment").unwrap()).unwrap();
            let inverse_ref: u64 = block_info.get_link_offset_normal("cc_cc_inverse").unwrap(); // zero if not inverse
            let cc_type_raw:u8 = block_info.get_data_value_first("cc_md_unit").unwrap();
            let mut cc_type = CcType::NotImplemented;
            let cc_val: Vec<u64> = block_info.get_data_value("cc_val").unwrap().clone().try_into().unwrap();
            let cc_val_count: u16 = block_info.get_data_value_first("cc_val_count").unwrap();
            match cc_type_raw {
                0 => { // one to one
                    cc_type = CcType::OneToOne;
                },

                1 if cc_val.len() == 2 => { // linear
                    cc_type = CcType::Linear((to_f64(cc_val[0]), to_f64(cc_val[1])));
                }

                2 if cc_val.len() ==6 => {
                    cc_type = CcType::Rational([to_f64(cc_val[0]), to_f64(cc_val[1]), to_f64(cc_val[2]),
                                        to_f64(cc_val[3]), to_f64(cc_val[4]), to_f64(cc_val[5])]);
                }
                
                4 if cc_val.len() == (cc_val_count*2u16) as usize => { // table
                    
                }


                _ => {
                    
                }
            }
                
             
            Ok(Conversion {
                name,
                unit,
                comment,
                inverse_ref,
                cc_type,
            })
        }
    }
}