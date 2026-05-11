use byteorder::{ByteOrder, LittleEndian, BigEndian};
use half::f16;
use indexmap::IndexMap;
use std::fmt::{self, Display};

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
        BigEndian::read_u16(buf)
    }
}

impl FromBeBytes for u32 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_u32(buf)
    }
}

impl FromBeBytes for u64 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_u64(buf)
    }
}

impl FromBeBytes for i8 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        buf[0] as i8
    }
}

impl FromBeBytes for i16 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_i16(buf)
    }
}

impl FromBeBytes for i32 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_i32(buf)
    }
}

impl FromBeBytes for i64 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        BigEndian::read_i64(buf)
    }
}

impl FromBeBytes for f16 {
    fn from_be_bytes(buf: &[u8]) -> Self{
        f16::from_be_bytes([buf[0], buf[1]])
    }
}

impl FromBeBytes for f32 {
    fn from_be_bytes(buf: &[u8]) -> Self {
        BigEndian::read_f32(buf)
    }
}

impl FromBeBytes for f64 {
    fn from_be_bytes(buf: &[u8]) -> Self {
        BigEndian::read_f64(buf)
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
        LittleEndian::read_u16(buf)
    }
}

impl FromLeBytes for u32 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_u32(buf)
    }
}

impl FromLeBytes for u64 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_u64(buf)
    }
}

impl FromLeBytes for i8 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        buf[0] as i8
    }
}

impl FromLeBytes for i16 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_i16(buf)
    }
}

impl FromLeBytes for i32 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_i32(buf)
    }
}

impl FromLeBytes for i64 {
    fn from_le_bytes(buf: &[u8]) -> Self{
        LittleEndian::read_i64(buf)
    }
}

impl FromLeBytes for f16 {
    fn from_le_bytes(buf: &[u8]) -> Self {
        f16::from_le_bytes([buf[0], buf[1]])
    }
}

impl FromLeBytes for f32 {
    fn from_le_bytes(buf: &[u8]) -> Self {
        LittleEndian::read_f32(buf)
    }
}

impl FromLeBytes for f64 {
    fn from_le_bytes(buf: &[u8]) -> Self {
        LittleEndian::read_f64(buf)
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

pub fn right_shift_bytes_inplace(bytes: &mut [u8], shift: usize) -> Result<(), &str> {
    if !(1..=7).contains(&shift) {
        Err("Shift must be between 1 and 7")
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

pub fn right_shift_bytes(bytes: &[u8], shift: u8) -> Result<Vec<u8>, &str> {
    if !(1..=7).contains(&shift) {
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

pub fn bytes_and_bits(bytes: &mut [u8], bits: u32) {
    // modify in place; this operation can not fail
    let num_of_bytes = (bits as f32 / 8.0).floor() as usize;
    let num_of_bits = bits % 8;
    if num_of_bytes < bytes.len() {
        bytes[num_of_bytes] &= 2_u8.pow(num_of_bits) - 1 ;
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

fn from_u8_vec(bytes: &[u8]) -> Result<Vec<u16>, &'static str> {
    if !bytes.len().is_multiple_of(2) {
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
        !matches!(self, &DataValue::CHAR(_) | &DataValue::STRINGS(_) | &DataValue::BYTEARRAY(_) | &DataValue::STRUCT(_) | &DataValue::MIXED(_))
    }

    pub fn is_strings(&self) -> bool {
        matches!(self, &DataValue::STRINGS(_))
    }

    /// Returns the number of elements in the data
    pub fn len(&self) -> usize {
        match self {
            DataValue::CHAR(_) => 1,
            DataValue::STRINGS(v) => v.len(),
            DataValue::BYTE(v) => v.len(),
            DataValue::UINT64(v) => v.len(),
            DataValue::UINT8(v) => v.len(),
            DataValue::INT8(v) => v.len(),
            DataValue::INT16(v) => v.len(),
            DataValue::UINT16(v) => v.len(),
            DataValue::INT32(v) => v.len(),
            DataValue::UINT32(v) => v.len(),
            DataValue::INT64(v) => v.len(),
            DataValue::REAL(v) => v.len(),
            DataValue::SINGLE(v) => v.len(),
            DataValue::FLOAT16(v) => v.len(),
            DataValue::STRUCT(m) => m.len(),
            DataValue::BYTEARRAY(v) => v.len(),
            DataValue::MIXED(v) => v.len(),
        }
    }

    /// Returns true if the data is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the data represents textual values.
    ///
    /// Returns `true` for `CHAR`, `STRINGS`, and `MIXED` variants.
    /// `MIXED` arises from value-to-text (CC type 7) or value-range-to-text (CC type 8)
    /// conversions, where each element may be either a resolved text label or a raw
    /// numeric fallback.
    pub fn is_text(&self) -> bool {
        matches!(self, DataValue::CHAR(_) | DataValue::STRINGS(_) | DataValue::MIXED(_))
    }

    /// Converts textual data to a `Vec<String>`, consuming `self`.
    ///
    /// - `CHAR(s)` → `Some(vec![s])`
    /// - `STRINGS(v)` → `Some(v)`
    /// - `MIXED(v)` → `Some(…)`, where each `StringOrReal::String(s)` becomes `s`
    ///   and each `StringOrReal::Real(f)` is formatted as its decimal representation.
    /// - All other variants → `None`.
    pub fn into_text(self) -> Option<Vec<String>> {
        match self {
            DataValue::CHAR(s) => Some(vec![s]),
            DataValue::STRINGS(v) => Some(v),
            DataValue::MIXED(v) => Some(
                v.into_iter()
                    .map(|sor| match sor {
                        StringOrReal::String(s) => s,
                        StringOrReal::Real(f) => format!("{}", f),
                    })
                    .collect(),
            ),
            _ => None,
        }
    }
}

macro_rules! fmt_vec_branch {
    ($f:expr, $name:expr, $v:expr) => {{
        write!($f, "{} ", $name)?;
        fmt_vec($f, $v)
    }};
}

fn fmt_vec<T: fmt::Display>(f: &mut fmt::Formatter<'_>, v: &[T]) -> fmt::Result {
    let n = v.len();
    if n == 0 {
        return write!(f, "[]");
    }
    if n <= 11 {
        write!(f, "[")?;
        for (i, item) in v.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, "]")
    } else {
        write!(f, "[")?;
        for (i, item) in v.iter().enumerate().take(10) {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, ", ..., {}", v[n - 1])?;
        write!(f, "] (len={})", n)
    }
}

// 为 DataValue 实现 Display trait，用于格式化输出
impl Display for DataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataValue::CHAR(s) => write!(f, "CHAR({})", s),
            DataValue::STRINGS(v) => fmt_vec_branch!(f, "STRINGS", v),
            DataValue::BYTE(v) => fmt_vec_branch!(f, "BYTE", v),
            DataValue::UINT64(v) => fmt_vec_branch!(f, "UINT64", v),
            DataValue::UINT8(v) => fmt_vec_branch!(f, "UINT8", v),
            DataValue::INT8(v) => fmt_vec_branch!(f, "INT8", v),
            DataValue::INT16(v) => fmt_vec_branch!(f, "INT16", v),
            DataValue::UINT16(v) => fmt_vec_branch!(f, "UINT16", v),
            DataValue::INT32(v) => fmt_vec_branch!(f, "INT32", v),
            DataValue::UINT32(v) => fmt_vec_branch!(f, "UINT32", v),
            DataValue::INT64(v) => fmt_vec_branch!(f, "INT64", v),
            DataValue::REAL(v) => fmt_vec_branch!(f, "REAL", v),
            DataValue::SINGLE(v) => fmt_vec_branch!(f, "SINGLE", v),
            DataValue::FLOAT16(v) => fmt_vec_branch!(f, "FLOAT16", v),
            DataValue::STRUCT(map) => {
                write!(f, "STRUCT {{ ")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            DataValue::BYTEARRAY(vv) => {
                write!(f, "BYTEARRAY(len={})", vv.len())
            }
            DataValue::MIXED(v) => {
                write!(f, "MIXED(len={})", v.len())
            }
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
            DataValue::BYTE(s) => Ok(s.into_iter().map(|f| f as f64).collect()),
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