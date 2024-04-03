pub mod conversion {
    use std::fs::File;
    use std::io::BufReader;
    use crate::block::BlockInfo;
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
        TableInt((Vec<f64>, Vec<f64>)),  // table with interpolation
        Table((Vec<f64>, Vec<f64>)), // table without interpolation
        ValueRange(Vec<f64>),
        ValueText((Vec<f64>, Vec<u64>)),   //first from cc_val, second from cc_ref
        ValueRangeText((Vec<f64>, Vec<u64>)), 
        Text2Value((Vec<u64>, Vec<f64>)),
        Text2Text(Vec<u64>),
        BitfieldText((Vec<u64>, Vec<u64>)),
        NotImplemented   // and error condition
    }

    fn to_f64(v: u64) -> f64 {
        // change to raw bytes
        let bytes = v.to_le_bytes();
        f64::from_le_bytes(bytes)  // return f64 value
    }

    impl Conversion {
        pub fn new(block_info: &BlockInfo, buf: &mut BufReader<File>) -> Result<Self, Box<dyn std::error::Error>> {
            let name: String = get_tx_data(buf, block_info.get_link_offset_normal("cc_tx_name").unwrap())
                                .unwrap_or("".to_string());
            let unit: String = get_text(buf, block_info.get_link_offset_normal("cc_md_unit").unwrap())
                                .unwrap_or("".to_string());
            let comment: String = get_text(buf, block_info.get_link_offset_normal("cc_md_comment").unwrap())
                                .unwrap_or("".to_string());
            let inverse_ref: u64 = block_info.get_link_offset_normal("cc_cc_inverse").unwrap(); // zero if not inverse
            let cc_type_raw:u8 = block_info.get_data_value_first("cc_type").unwrap();
            let mut cc_type = CcType::NotImplemented;
            let cc_val: Vec<u64> = block_info.get_data_value("cc_val").unwrap().clone().try_into().unwrap();
            let cc_val_count: u16 = block_info.get_data_value_first("cc_val_count").unwrap();
            let cc_ref: Vec<u64> = block_info.get_link_offset_vec("cc_ref")
                                            .unwrap_or(Vec::new()).clone().try_into().unwrap(); // could be Nil
            let cc_ref_count: u16 = block_info.get_data_value_first("cc_ref_count").unwrap();
            match cc_type_raw {
                0 => { // one to one
                    cc_type = CcType::OneToOne;
                },

                1 if cc_val.len() == 2 => { // linear
                    cc_type = CcType::Linear((to_f64(cc_val[0]), to_f64(cc_val[1])));
                },

                2 if cc_val.len() == 6 => {
                    cc_type = CcType::Rational([to_f64(cc_val[0]), to_f64(cc_val[1]), to_f64(cc_val[2]),
                                        to_f64(cc_val[3]), to_f64(cc_val[4]), to_f64(cc_val[5])]);
                },
                
                4|5 if cc_val.len() == (cc_val_count) as usize && cc_val.len() % 2 == 0 => { // table
                    let mut key: Vec<f64> = Vec::new();
                    let mut value: Vec<f64> = Vec::new();
                    for i in 0..cc_val_count/2 {
                        key.push(to_f64(cc_val[i as usize*2]));
                        value.push(to_f64(cc_val[i as usize*2+1]));
                    }
                    if cc_type_raw == 4 {
                        cc_type = CcType::TableInt((key, value));
                    } else {
                        cc_type = CcType::Table((key, value));
                    }
                },
                6 if cc_val.len() == (cc_val_count) as usize => { // value range
                    let value: Vec<f64> = cc_val.into_iter().map(|v| to_f64(v)).collect();  // consumed cc_val
                    cc_type = CcType::ValueRange(value);
                },
                7 if cc_val.len() == cc_val_count as usize && cc_ref.len() == cc_ref_count as usize && cc_ref_count == cc_val_count + 1 => { // ValueText
                    let key = cc_val.into_iter().map(|v| to_f64(v)).collect();
                    cc_type = CcType::ValueText((key, cc_ref)); // key value; value stored in tx block which cc_ref points at
                },

                8 if cc_val.len() == (cc_val_count) as usize && cc_ref.len() == (cc_val_count/2+1) as usize => { // value range with text
                    let mut value: Vec<f64> = Vec::new();
                    (0..cc_val_count/2).for_each(|i:u16| {
                        value.push(to_f64(cc_val[i as usize*2]));   // min
                        value.push(to_f64(cc_val[i as usize*2 + 1]));  //max
                             //corresponding text
                    });
                    cc_type = CcType::ValueRangeText((value, cc_ref));  // cc_ref moved here
                },

                9 if cc_val.len() == (cc_ref_count+1) as usize && cc_ref.len() == (cc_ref_count) as usize => { // text to value
                    let mut value: Vec<f64> = Vec::new();
                    let mut text: Vec<u64> = Vec::new();
                    (0..cc_ref_count).for_each(|i:u16| {
                        text.push(cc_ref[i as usize]);     //corresponding text
                        value.push(to_f64(cc_val[i as usize]));   //value
                    });
                    value.push(to_f64(cc_val[(cc_ref_count) as usize]));
                    cc_type = CcType::Text2Value((text, value));
                },

                10 if cc_ref.len() == cc_ref_count as usize => {
                    cc_type = CcType::Text2Text(cc_ref);  // move ownship from cc_ref to cc_type, cc_ref can not be used anymore
                },

                11 if cc_ref_count == cc_val_count && cc_val.len() == cc_val_count as usize && cc_ref.len() == cc_ref_count as usize => { // bitfield text
                    cc_type = CcType::BitfieldText((cc_val, cc_ref));
                }
                _ => {
                    println!("cc block {} has not support cc_type", name);
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

        pub fn get_unit(&self) -> &str {
            &self.unit
        }

        pub fn get_comment(&self) -> &str {
            &self.comment
        }

        pub fn is_inverse(&self) -> bool {
            self.inverse_ref != 0
        }

        pub fn transform_value<T, U>(&self, int: T) -> U 
        where T: Into<f64>, U: From<f64>{  // intermediate calculation use f64
            let inp: f64 = int.into();
            match &self.cc_type {
                CcType::OneToOne => {
                    U::from(inp)
                },
                CcType::Linear(v) => {
                    U::from(v.0 + v.1 * inp)
                },
                CcType::Rational(v) => {
                    let numerator = v[0] * inp * inp + v[1]* inp + v[2];
                    let denominator = v[3] * inp * inp + v[4] * inp + v[5];
                    U::from(numerator / denominator)
                },
                CcType::TableInt((index, value)) => {
                    let mut right_ind = 0;
                    while inp >= index[right_ind] && right_ind < index.len() {
                        right_ind += 1;
                    };
                    if right_ind == 0 {
                        U::from(value[0])
                    } else if right_ind == index.len() {
                        U::from(value[value.len()-1])
                    } else {
                        let left_val = value[right_ind-1];
                        let right_val = value[right_ind];
                        let left_ind_val = index[right_ind-1];
                        let right_ind_val = index[right_ind];
                        let ratio = (right_val - left_val) / (right_ind_val - left_ind_val);
                        U::from(left_val + ratio * (inp - left_ind_val))
                    }
                },
                CcType::Table((index, value)) => {
                    let mut left_ind = 0;
                    while inp >= index[left_ind] && left_ind < index.len() {
                        left_ind += 1;
                    }
                    U::from(value[left_ind])
                },
                CcType::ValueRange(value) => {
                    let default_value = value.last().unwrap();
                    let mut left_ind = 0;
                    while inp >= value[left_ind] && left_ind < value.len()-1 {
                        left_ind += 3;
                    };
                    if left_ind >= value.len() - 1 {
                        U::from(default_value.to_owned())
                    } else {
                        U::from(value[left_ind+2])
                    }
                },
                _ => {
                    panic!("cc block {} has not support cc_type", self.name);
                }
            }
        }

        pub fn convert_to_text<T>(&self, buf: &mut BufReader<File>,int: T) -> Result<String, Box<dyn std::error::Error>> 
        where T: Into<f64> {
            let inp: f64 = int.into();
            match &self.cc_type {
                CcType::ValueRangeText((value, ref_text)) => {
                    let mut left_ind = 0;
                    let default_value: String = get_text(buf, ref_text[ref_text.len()-1])
                                                .unwrap_or("".to_string());
                    while left_ind < value.len() && inp >= value[left_ind] {
                        left_ind += 2;
                    }
                    if left_ind >= 2 {
                        let left_val = value[left_ind-2];
                        let right_val = value[left_ind-1];
                        if inp <= right_val && inp >= left_val {
                            Ok(get_text(buf, ref_text[left_ind/2-1]).unwrap_or(default_value))
                        } else {
                            Ok(default_value)
                        }
                    } else {
                        Ok(default_value)
                    }
                },
                CcType::ValueText((value, ref_text)) => {
                    let mut left_ind:usize = 0;
                    let default_value: String = get_text(buf, ref_text[ref_text.len()-1])
                                                .unwrap_or("".to_string());
                    while left_ind < value.len() && inp != value[left_ind] {
                        left_ind += 1;
                    }
                    if left_ind < value.len() {
                        Ok(get_text(buf, ref_text[left_ind]).unwrap_or(default_value))
                    } else {
                        Ok(default_value)
                    }
                },
                _ => {
                        panic!("cc block {} has not support cc_type", self.name);
                    }
            }
        }
    }

    
}


#[cfg(test)]
pub mod conversion_test {
    use crate::components::cc::conversion::*;
    use crate::block::*;
    use rust_embed::RustEmbed;
    use std::io::{BufReader, Write};
    use std::fs::File;
    use std::sync::Mutex;
    use rstest::*;

    #[derive(RustEmbed)]
    #[folder = "test/"]
    #[prefix = "test/"]
    struct Asset;

    #[fixture]
    #[once]
    fn buffer() -> Mutex<BufReader<File>> {
        let file_data = Asset::get("test/1.mf4").unwrap();
        let mut new_file = File::create("temp.mf4").unwrap();
        new_file.write(file_data.data.as_ref()).unwrap();
        let file = File::open("temp.mf4").unwrap();
        let buf= BufReader::new(file);
        Mutex::new(buf)
    }

    #[fixture]
    #[once]
    fn cc_desc() -> BlockDesc {
        let dg_toml_file = Asset::get("test/cc.toml").unwrap();
        let toml_str = String::from_utf8(dg_toml_file.data.as_ref().to_vec()).unwrap();
        toml::from_str(toml_str.as_str()).unwrap()
    }

    #[rstest]
    fn test_cc1(buffer: &Mutex<BufReader<File>>, cc_desc: &'static BlockDesc) {
        let offset = 0x52E8;
        let mut buf = buffer.lock().unwrap();
        let cc_info: BlockInfo = cc_desc.try_parse_buf(&mut buf, offset).unwrap();
        let cc = Conversion::new(&cc_info, &mut buf).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.transform_value::<f64, f64>(1000.0), 2000.0);   // linear with p2 = 2
    }

    #[rstest]
    fn test_cc2(buffer: &Mutex<BufReader<File>>, cc_desc: &'static BlockDesc) {
        let offset = 0x5348;
        let mut buf = buffer.lock().unwrap();
        let cc_info: BlockInfo = cc_desc.try_parse_buf(&mut buf, offset).unwrap();
        let cc = Conversion::new(&cc_info, &mut buf).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.transform_value::<f64, f64>(1000.0), 1000.0);   // linear with p2 = 1
    }

    #[rstest]
    fn test_cc3(buffer: &Mutex<BufReader<File>>, cc_desc: &'static BlockDesc) {
        let offset = 0x53A8;
        let mut buf = buffer.lock().unwrap();
        let cc_info: BlockInfo = cc_desc.try_parse_buf(&mut buf, offset).unwrap();
        let cc = Conversion::new(&cc_info, &mut buf).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "hundredfive\0");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.convert_to_text::<f64>(&mut buf, 0.5).unwrap(), "Zero_to_one\0".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 1u8).unwrap(), "Zero_to_one\0".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 2.5).unwrap(), "two_to_three\0".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 105).unwrap(), "hundredfive\0".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 105.1).unwrap(), "".to_string());
        assert_eq!(cc.convert_to_text(&mut buf, 15.1).unwrap(), "fourteen_to_seventeen\0".to_string());
    }

    #[rstest]
    fn test_cc4(buffer: &Mutex<BufReader<File>>, cc_desc: &'static BlockDesc) {
        let offset = 0x5508;
        let mut buf = buffer.lock().unwrap();
        let cc_info: BlockInfo = cc_desc.try_parse_buf(&mut buf, offset).unwrap();
        let cc = Conversion::new(&cc_info, &mut buf).unwrap();

        println!("{:?}", cc);
        assert_eq!(cc.get_unit(), "unknown signal type\0");
        assert_eq!(cc.get_comment(), "");
        assert!(!cc.is_inverse());
        assert_eq!(cc.convert_to_text(&mut buf, 15.1).unwrap(), "unknown signal type\0".to_string());   // linear with p2 = 1
        assert_eq!(cc.convert_to_text(&mut buf, 3).unwrap(), "Sinus\0".to_string());   // linear with p2 = 1
        assert_eq!(cc.convert_to_text(&mut buf, 2).unwrap(), "Square\0".to_string());   // linear with p2 = 1
        assert_eq!(cc.convert_to_text(&mut buf, 1).unwrap(), "SawTooth\0".to_string());   // linear with p2 = 1
    }
}