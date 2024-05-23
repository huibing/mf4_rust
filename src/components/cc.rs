pub mod conversion {
    use std::fs::File;
    use std::io::BufReader;
    use crate::block::BlockInfo;
    use crate::data_serde::{DataValue, StringOrReal};
    use crate::parser::{get_clean_text, get_block_desc_by_name, peek_block_type};
    use evalexpr::*;

    #[derive(Debug, Clone, Default)]
    pub struct Conversion 
    {
        name: String,
        unit: String,
        comment: String,
        inverse_ref: u64, // TODO: implement inverse conversion
        cc_type: CcType,
    } 
    
    #[derive(Debug, Clone, Default)]
    pub enum CcType {
        #[default]
        OneToOne,
        Linear((f64, f64)),
        Rational([f64; 6]),
        Algebraic(String),
        TableInt((Vec<f64>, Vec<f64>)),  // table with interpolation
        Table((Vec<f64>, Vec<f64>)), // table without interpolation
        ValueRange(Vec<f64>),
        Value2Text((Vec<f64>, Vec<TextOrScale>)),   //first from cc_val, second from cc_ref
        ValueRange2Text((Vec<f64>, Vec<TextOrScale>)), 
        Text2Value((Vec<u64>, Vec<f64>)),
        Text2Text(Vec<u64>),
        BitfieldText((Vec<u64>, Vec<u64>)),
        NotImplemented   // and error condition
    }

    #[derive(Debug, Clone)]
    pub enum TextOrScale {   // only for Value2Text and ValueRange2Text
        Text(String),
        Scale(Conversion)
    }

    impl CcType {
        pub fn is_num(&self) -> bool {
            // the target type of this conversion is numeric
            match self {
                CcType::OneToOne => true,
                CcType::Linear(_) => true,
                CcType::Rational(_) => true,
                CcType::TableInt(_) => true,
                CcType::Table(_) => true,
                CcType::ValueRange(_) => true,
                CcType::Text2Value(_) => true,
                CcType::Algebraic(_) => true,
                _ => false
            }
        }
    }

    fn to_f64(v: u64) -> f64 {
        // change to raw bytes
        let bytes = v.to_le_bytes();
        f64::from_le_bytes(bytes)  // return f64 value
    }

    impl Conversion {
        pub fn new(buf: &mut BufReader<File>, offset: u64) -> Result<Self, Box<dyn std::error::Error>> {
            if offset == 0 {
                return Ok(Self::default())    // allows default
            }
            let cc_desc = get_block_desc_by_name("CC".to_string()).unwrap();
            let block_info: BlockInfo = cc_desc.try_parse_buf(buf, offset).unwrap();
            let name: String = get_clean_text(buf, block_info.get_link_offset_normal("cc_tx_name").unwrap())
                                .unwrap_or("".to_string());
            let unit: String = get_clean_text(buf, block_info.get_link_offset_normal("cc_md_unit").unwrap())
                                .unwrap_or("".to_string());
            let comment: String = get_clean_text(buf, block_info.get_link_offset_normal("cc_md_comment").unwrap())
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
                3 if cc_ref_count == 1 && cc_ref.len() == 1 => { // text2value
                    let text = get_clean_text(buf, cc_ref[0])?;
                    cc_type = CcType::Algebraic(text); 
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
                7 if cc_val.len() == cc_val_count as usize && cc_ref.len() == cc_ref_count as usize && cc_ref_count == cc_val_count + 1 => { // Value2Text
                    let key: Vec<f64> = cc_val.into_iter().map(|v| to_f64(v)).collect();
                    let mut text: Vec<TextOrScale> = Vec::new();
                    for link in cc_ref.into_iter() {
                        let block_type = peek_block_type(buf, link).unwrap_or("TX".to_string());  // handle Nil
                        match block_type.as_str() {
                            "TX" => text.push(TextOrScale::Text(get_clean_text(buf, link).unwrap_or("".to_string()))),
                            "CC" => {
                                let conversion = Conversion::new(buf, link).unwrap_or(Conversion::default());
                                text.push(TextOrScale::Scale(conversion));
                            }
                            _ => return Ok(Conversion::default())   // error handling: no panic no err, just fall back to default
                        }
                    }
                    cc_type = CcType::Value2Text((key, text)); // key value; value stored in tx block which cc_ref points at
                },

                8 if cc_val.len() == (cc_val_count) as usize && cc_ref.len() == (cc_val_count/2+1) as usize => { // value range with text
                    let mut value: Vec<f64> = Vec::new();
                    (0..cc_val_count/2).for_each(|i:u16| {
                        value.push(to_f64(cc_val[i as usize*2]));   // min
                        value.push(to_f64(cc_val[i as usize*2 + 1]));  //max
                             //corresponding text
                    });
                    let mut text: Vec<TextOrScale> = Vec::new();   // same as Value2Text
                    for link in cc_ref.into_iter() {
                        let block_type = peek_block_type(buf, link).unwrap_or("TX".to_string());  // handle error later
                        match block_type.as_str() {
                            "TX" => text.push(TextOrScale::Text(get_clean_text(buf, link).unwrap_or("".to_string()))),
                            "CC" => {
                                let conversion = Conversion::new(buf, link).unwrap_or(Conversion::default());
                                text.push(TextOrScale::Scale(conversion));
                            }
                            _ => return Ok(Conversion::default())   // error handling: no panic no err, just fall back to default
                        }
                    }
                    cc_type = CcType::ValueRange2Text((value, text));  // cc_ref moved here
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
                    println!("cc block {} has not support cc_type type {}", name, cc_type_raw);
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

        pub fn get_cc_name(&self) -> &str {
            &self.name
        }

        pub fn get_cc_type(&self) -> &CcType {
            &self.cc_type
        }

        pub fn convert_num_value<T, U>(&self, int: T) -> U 
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
                    let mut right_ind: usize = 0;
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
                    if inp <= index[0] {
                        U::from(value[0])
                    } else if inp >= index[index.len()-1] {
                        U::from(value[value.len()-1])
                    } else {
                        let mut right_ind = 0;
                        while right_ind < index.len() && inp >= index[right_ind]{
                            right_ind += 1;
                        };
                        let left_val = value[right_ind-1];
                        U::from(left_val)
                    }
                },
                CcType::ValueRange(value) => {
                    let default_value = value.last().unwrap();
                    let mut left_ind = 0;
                    while left_ind < value.len()-1 && inp >= value[left_ind] {
                        left_ind += 3;
                    };
                    if left_ind == 0 || left_ind >= value.len()-1 {
                        U::from(default_value.to_owned())
                    } else {
                        U::from(value[left_ind-1])
                    }
                },
                CcType::Algebraic(text) => {
                    let context = context_map! {"X" => inp}.unwrap();
                    let value = eval_float_with_context(&text, &context).unwrap();
                    U::from(value)
                }
                _ => {
                    panic!("cc block {} has not support cc_type", self.name);
                }
            }
        }

        pub fn convert_to_mix<T>(&self, buf: &mut BufReader<File>,int: T) -> Result<StringOrReal, Box<dyn std::error::Error>> 
        where T: Into<f64> {
            let inp: f64 = int.into();
            match &self.cc_type {
                CcType::ValueRange2Text((value, ref_text)) => {
                    let mut left_ind: usize = 0;
                    while left_ind < value.len() && inp >= value[left_ind] {
                        if inp <= *value.get(left_ind+1).unwrap_or(&f64::MIN) { // found a match
                            let item: TextOrScale = ref_text[left_ind/2].clone();
                            return Self::to_mix(&item, inp, buf)
                        } else {
                            left_ind += 2;
                        }
                    }
                    let default: TextOrScale = ref_text[ref_text.len()-1].clone();
                    Self::to_mix(&default, inp, buf)          
                },
                CcType::Value2Text((value, ref_text)) => {
                    let mut left_ind:usize = 0;
                    let default_value: TextOrScale= ref_text[ref_text.len() - 1].clone();
                    while left_ind < value.len() && inp != value[left_ind] {
                        left_ind += 1;
                    }
                    if left_ind < value.len() { // found a match
                        let item: TextOrScale = ref_text[left_ind].clone();
                        Self::to_mix(&item, inp, buf)
                    } else {  // fall back to default
                        Self::to_mix(&default_value, inp, buf)
                    }
                },
                _ => {
                        panic!("cc block {} has not support cc_type", self.name);
                    }
            }
        }

        fn to_mix<T>(conv: &TextOrScale, inp: T, buf: &mut BufReader<File>) -> Result<StringOrReal,  Box<dyn std::error::Error>> 
        where T: Into<f64>{
            match conv {
                TextOrScale::Text(text) => Ok(StringOrReal::String(text.clone())),
                TextOrScale::Scale(conv) => {
                    if conv.cc_type.is_num() {
                        let num: f64 = conv.convert_num_value(inp);
                        Ok(StringOrReal::Real(num))
                    } else {
                        let mix: Result<StringOrReal, Box<dyn std::error::Error>> = conv.convert_to_mix(buf, inp);
                        mix   //  for debug purpose
                    }
                },
            }
        }

        pub fn convert_from_text(&self, buf: &mut BufReader<File>, inp: &Vec<String>) -> Result<DataValue, Box<dyn std::error::Error>> {
            match &self.cc_type {
                CcType::Text2Value((text, value)) => {
                    let mut ref_text: Vec<String> = Vec::new();
                    for t in text.iter() {
                        let clean_text: String = get_clean_text(buf, *t)?;
                        ref_text.push(clean_text);
                    }
                    let mut result: Vec<f64> = Vec::new();
                    let default_value: f64 = value[value.len()-1];
                    for inp_str in inp.iter() {
                        let mut found = false;
                        for (ref_t, t) in ref_text.iter().zip(value.iter()) {
                            if ref_t == inp_str {
                                result.push(*t);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            result.push(default_value);
                        }
                    }
                    Ok(DataValue::REAL(result))  // default value; last element in values
                },
                CcType::Text2Text(text) => {
                    let total_num: usize = text.len();
                    if total_num % 2 == 0 {
                        Err("text2text cc block has odd number of elements".into())
                    } else {
                        let half_num = total_num / 2;
                        let mut ref_text = Vec::new();
                        let mut value_text = Vec::new();
                        for i in 0..half_num {
                            ref_text.push(get_clean_text(buf, text[i*2].to_owned())?);
                            value_text.push(get_clean_text(buf, text[i*2+1].to_owned())?);
                        }
                        let default_value: String = get_clean_text(buf, text[total_num-1].to_owned())?;
                        let mut result: Vec<String> = Vec::new();
                        for inp_str in inp.iter() {
                            let mut found = false;
                            for (ref_t, t) in ref_text.iter().zip(value_text.iter()) {
                                if ref_t == inp_str {
                                    result.push(t.to_owned());
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                result.push(default_value.clone());
                            }
                        }
                        Ok(DataValue::STRINGS(result))  
                    }
                },
                CcType::OneToOne => Ok(DataValue::STRINGS(inp.clone())),
                other_type => {
                    Err(format!("{:?} does not support from text conversions", other_type).into())
                },
            }
        }
    }
}
