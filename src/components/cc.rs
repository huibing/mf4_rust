pub mod conversion {
    use std::fs::File;
    use std::io::BufReader;
    use crate::block::BlockInfo;
    use crate::parser::get_tx_data;


    pub struct Conversion 
    {
        name: String,
        unit: String,
        comment: String,
        inverse: bool,
        cc_type: CcType,
    } 
    

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
        BitfieldText((Vec<u64>, Vec<u64>))
    }

    impl Conversion {
        pub fn new(block_info: &BlockInfo, buf: &mut BufReader<File>) -> Self {
            
        }
    }
}