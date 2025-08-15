use byteorder::{ByteOrder, LittleEndian, BigEndian};
use half::f16;
use indexmap::IndexMap;

pub struct UTF16String {
    pub inner: String,
}


pub trait FromLeBytes {
    fn from_le_bytes(buf: &[u8]) -> Self;
}

pub trait FromBeBytes {
    fn from_be_bytes(buf: &[u8]) -> Self;
}

/* Big Endian */
impl FromBeBytes for u8 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        buf[0]
    }
}

impl FromBeBytes for u16 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_u16(&buf)
    }
}

impl FromBeBytes for u32 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_u32(&buf)
    }
}

impl FromBeBytes for u64 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_u64(&buf)
    }
}

impl FromBeBytes for i8 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        buf[0] as i8
    }
}

impl FromBeBytes for i16 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_i16(&buf)
    }
}

impl FromBeBytes for i32 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_i32(&buf)
    }
}

impl FromBeBytes for i64 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_i64(&buf)
    }
}

impl FromBeBytes for f16 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        f16::from_be_bytes([buf[0], buf[1]])
    }
}

impl FromBeBytes for f32 {
    fn from_be_bytes(buf: &[u8]) -> Self {
        BigEndian::read_f32(&buf)
    }
}

impl FromBeBytes for f64 {
    fn from_be_bytes(buf: &[u8]) -> Self {
        BigEndian::read_f64(&buf)
    }
}

impl FromBeBytes for String {
    fn from_be_bytes(buf: &[u8]) -> Self{
        let mut new_arr: Vec<u8> = buf.to_vec();    // has to copy because we are going to reverse it
        reverse_bytes_array(&mut new_arr);
        String::from_utf8(new_arr).unwrap_or("ERROR during parsing BE STRING".to_string())
    }
}

impl FromBeBytes for UTF16String {
    fn from_be_bytes(buf: &[u8]) -> Self{
        let mut new_arr: Vec<u8> = buf.to_vec();
        reverse_bytes_array(&mut new_arr);
        let u16s = from_u8_vec(&new_arr).unwrap();
        UTF16String {
            inner: String::from_utf16_lossy(&u16s[..])
        }
    }
}

/* Little Endian */
impl FromLeBytes for u8 {
    fn from_le_bytes(buf: &[u8]) -> Self {
        buf[0]
    }
}

impl FromLeBytes for u16 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_u16(&buf)
    }
}

impl FromLeBytes for u32 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_u32(&buf)
    }
}

impl FromLeBytes for u64 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_u64(&buf)
    }
}

impl FromLeBytes for i8 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        buf[0] as i8
    }
}

impl FromLeBytes for i16 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_i16(&buf)
    }
}

impl FromLeBytes for i32 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_i32(&buf)
    }
}

impl FromLeBytes for i64 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_i64(&buf)
    }
}

impl FromLeBytes for f16 {
    fn from_le_bytes(buf: &[u8]) -> Self {
        f16::from_le_bytes([buf[0], buf[1]])
    }
}

impl FromLeBytes for f32 {
    fn from_le_bytes(buf: &[u8]) -> Self {
        LittleEndian::read_f32(&buf)
    }
}

impl FromLeBytes for f64 {
    fn from_le_bytes(buf: &[u8]) -> Self {
        LittleEndian::read_f64(&buf)
    }
}

impl FromLeBytes for String {
    fn from_le_bytes(buf: &[u8]) -> Self{
        String::from_utf8(buf.to_vec())
            .unwrap_or("ERROR during parsing LB STRING".to_string())
    }
}

impl FromLeBytes for UTF16String {
    fn from_le_bytes(buf: &[u8]) -> Self{
        let new_arr: Vec<u8> = buf.to_vec();
        let u16s = from_u8_vec(&new_arr).unwrap();
        UTF16String {
            inner: String::from_utf16_lossy(&u16s[..])
        }
    }
}

pub fn parse_le_value<T>(cur: &[u8]) -> T
    where T: FromLeBytes {
        T::from_le_bytes(cur)
    }


pub fn parse_be_value<T>(cur: &[u8]) -> T
    where T: FromBeBytes {
        T::from_be_bytes(cur)
    }

pub fn right_shift_bytes_inplace(bytes: &mut Vec<u8>, shift: usize) -> Result<(), &str> {
    if shift > 7 || shift < 1 {
        return Err("Shift must be between 1 and 7");
    } else {
        let mut carry = 0u8;
        for byte in bytes.iter_mut().rev() {
            let shift_byte = (*byte >> shift) | carry;
            carry = *byte << (8 - shift);
            *byte = shift_byte;
        }
        Ok(())
    }
}

pub fn right_shift_bytes(bytes: &Vec<u8>, shift: u8) -> Result<Vec<u8>, &str> {
    if shift>7 || shift < 1 {
        return Err("Shift must be between 1 and 7");
    }
    let mut new = Vec::new();
    let mut carry = 0u8;
    for byte in bytes.iter().rev() {
        let shift_byte = (*byte >> shift) | carry;
        carry = *byte << (8 - shift);
        new.insert(0, shift_byte);
    }
    Ok(new)
}

pub fn bytes_and_bits(bytes: &mut Vec<u8>, bits: u32) {
    // modify in place; this operation can not fail
    let num_of_bytes = (bits as f32 / 8.0).floor() as usize;
    let num_of_bits = bits % 8;
    if num_of_bytes < bytes.len() {
        bytes[num_of_bytes] = bytes[num_of_bytes] & (2_u8.pow(num_of_bits as u32) - 1);
        (num_of_bytes + 1..bytes.len()).for_each(|i| bytes[i] = 0);
    } //  nothing needs to be done if bits is larger than the bytes array
}
pub fn reverse_bytes_array(arr: &mut [u8]) {
    // Reverse the order of the bytes in the array; to decode Be::Utf-16 stirng
    let mut left: usize = 0;
    let mut right: usize = arr.len() - 1;
    while left < right {
        arr.swap(left, right);
        left += 1;
        right -= 1;
    }
}

fn from_u8_vec(bytes: &Vec<u8>) -> Result<Vec<u16>, &'static str> {
    if bytes.len() % 2 != 0 {
        return Err("Length of bytes must be even");
    }
    let result: Vec<u16> = bytes.chunks(2)
                                .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                                .collect();
    Ok(result)
}
#[derive(Debug, PartialEq, Clone)]
pub enum StringOrReal {  // for Value2Text and ValueRange2Text conversions only
    String(String),
    Real(f64),
}

impl StringOrReal {
    pub fn into_string(self) -> Result<String, &'static str> {
        match self {
            StringOrReal::String(s) => Ok(s),
            StringOrReal::Real(_) => Err("Can not convert real to string")
        }
    }
}


#[derive(Debug, PartialEq, Clone)]
pub enum DataValue {
    CHAR(String),
    STRINGS(Vec<String>),
    BYTE(Vec<u8>),
    UINT64(Vec<u64>),
    UINT8(Vec<u8>),
    INT8(Vec<i8>),
    INT16(Vec<i16>),
    UINT16(Vec<u16>),
    INT32(Vec<i32>),
    UINT32(Vec<u32>), 
    INT64(Vec<i64>),
    REAL(Vec<f64>),
    SINGLE(Vec<f32>),
    FLOAT16(Vec<f16>),
    STRUCT(IndexMap<String, DataValue>),   
    BYTEARRAY(Vec<Vec<u8>>),
    MIXED(Vec<StringOrReal>)
}



impl DataValue {
    pub fn is_num(&self) -> bool {
        match self {
            &DataValue::CHAR(_) | &DataValue::STRINGS(_) | &DataValue::BYTEARRAY(_) | &DataValue::STRUCT(_) => false,
            _ => true
        }
    }

    pub fn is_strings(&self) -> bool {
        match self {
            &DataValue::STRINGS(_) => true,
            _ => false
        }
    }
}

impl TryFrom<DataValue> for String {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::CHAR(s) => Ok(s),
            _ => Err("DataValue is not a CHAR")
        }
    }
}

impl TryFrom<DataValue> for Vec<u8> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::BYTE(s) => Ok(s),
            DataValue::UINT8(s) => Ok(s),
            DataValue::CHAR(s) => Ok(s.into_bytes()),
            _ => Err("DataValue is not a uint8 or byte")
        }
    }
}

impl TryFrom<DataValue> for Vec<u64> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::UINT64(s) => Ok(s),
            _ => Err("DataValue is not a uint64")
        }
    }
}

impl TryFrom<DataValue> for Vec<i16> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::INT16(s) => Ok(s),
            _ => Err("DataValue is not a int16")
        }
    }
}

impl TryFrom<DataValue> for Vec<u16> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::UINT16(s) => Ok(s),
            _ => Err("DataValue is not a uint16")
        }
    }
}

impl TryFrom<DataValue> for Vec<i32> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::INT32(s) => Ok(s),
            _ => Err("DataValue is not a int32")
        }
    }
}

impl TryFrom<DataValue> for Vec<u32> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::UINT32(s) => Ok(s),
            _ => Err("DataValue is not a uint32")
        }
    }
}

impl TryFrom<DataValue> for Vec<i64> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::INT64(s) => Ok(s),
            _ => Err("DataValue is not a int64")
        }
    }
}

// special cases for f64; need a conveient way to convert any num to f64
impl TryFrom<DataValue> for Vec<f64> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::REAL(s) => Ok(s),
            DataValue::FLOAT16(s) => Ok(s.into_iter().map(|f| f.to_f64()).collect()),
            DataValue::SINGLE(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::INT16(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::UINT16(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::INT32(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::UINT32(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::INT64(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::UINT64(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::INT8(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            DataValue::UINT8(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
            _ => Err("DataValue is not a float64")
        }
    }
}

impl TryFrom<DataValue> for Vec<f32> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::SINGLE(s) => Ok(s),
            DataValue::REAL(s) => Ok(s.iter().map(|f| *f as f32).collect()),
            DataValue::UINT32(s) => Ok(s.into_iter().map(|f| f as f32).collect()),
            DataValue::UINT64(s) => Ok(s.into_iter().map(|f| f as f32).collect()),
            _ => Err("DataValue is not a float32")
        }
    }
}

impl TryFrom<DataValue> for Vec<f16> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::FLOAT16(s) => Ok(s),
            _ => Err("DataValue is not a float16")
        }
    }
}

impl TryFrom<DataValue> for Vec<String> {
    type Error = &'static str;
    fn try_from(value: DataValue) -> Result<Self, Self::Error> {
        match value {
            DataValue::STRINGS(s) => Ok(s),
            _ => Err("DataValue is not a float16")
        }
    }
}




#[cfg(test)]
pub mod serde_tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_u8_from_le_bytes() {
        let cursor = vec![0x12u8];
        assert_eq!(0x12u8, parse_le_value::<u8>(&cursor));
    }

    #[rstest]
    fn test_u16_from_le_bytes() {
        let cursor = vec![0x12u8, 0x34];
        assert_eq!(0x3412u16, parse_le_value::<u16>(&cursor));
    }

    #[rstest]
    fn test_f32_from_le_bytes() {
        let cursor = vec![0x00u8, 0x00, 0x48, 0x41];
        assert_eq!(12.5f32, parse_le_value::<f32>(&cursor));
    }

    #[rstest]
    fn test_f32_from_be_bytes() {
        let cursor = vec![0x41u8, 0x48, 0x00, 0x00];
        assert_eq!(12.5f32, parse_be_value::<f32>(&cursor));
    }

    #[rstest]
    fn test_f64_from_be_bytes() {
        let cursor = vec![0x41u8, 0x48, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(3145728.0f64, parse_be_value::<f64>(&cursor));
    }

    #[rstest]
    fn test_right_shift_fn() {
        let mut a: Vec<u8> = vec![0x01u8, 0x02, 0x03, 0x04];
        let b = vec![0x01u8, 0x02, 0x03, 0x04];
        right_shift_bytes_inplace(&mut a, 3).unwrap();
        assert_eq!(vec![64, 96, 128, 0], a);
        let new = right_shift_bytes(&b, 3).unwrap();
        assert_eq!(vec![64, 96, 128, 0], new);
    }

    #[rstest]
    fn test_bytes_fn() {
        let mut a: Vec<u8> = vec![0x01u8, 0x02, 0xff, 0xff];
        bytes_and_bits(&mut a, 23);
        assert_eq!(vec![0x01u8, 0x02, 0x7f, 0x00], a);
        reverse_bytes_array(&mut a);
        assert_eq!(vec![0x00u8, 0x7f, 0x02, 0x01], a);
    }
}