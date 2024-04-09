use std::io::{Read, Cursor};
use byteorder::{ByteOrder, LittleEndian, BigEndian};

pub trait FromLeBytes {
    fn from_le_bytes<T>(buf: &mut T) -> Self
        where T: Read;
}

pub trait FromBeBytes {
    fn from_be_bytes<T>(buf: &mut T) -> Self
        where T: Read;

}

/* Big Endian */
impl FromBeBytes for u8 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 1];
        bytes.read_exact(&mut buf).unwrap();
        buf[0]
    }
}

impl FromBeBytes for u16 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 2];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_u16(&buf)
    }
}

impl FromBeBytes for u32 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 4];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_u32(&buf)
    }
}

impl FromBeBytes for u64 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 8];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_u64(&buf)
    }
}

impl FromBeBytes for i8 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 1];
        bytes.read_exact(&mut buf).unwrap();
        i8::from_be_bytes(buf)
    }
}

impl FromBeBytes for i16 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 2];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_i16(&buf)
    }
}

impl FromBeBytes for i32 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 4];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_i32(&buf)
    }
}

impl FromBeBytes for i64 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 8];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_i64(&buf)
    }
}

impl FromBeBytes for f32 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self
    where T: Read {
        let mut buf = [0u8; 4];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_f32(&buf)
    }
}

impl FromBeBytes for f64 {
    fn from_be_bytes<T>(bytes: &mut T) -> Self
    where T: Read {
        let mut buf = [0u8; 8];
        bytes.read_exact(&mut buf).unwrap();
        BigEndian::read_f64(&buf)
    }
}

/* Little Endian */
impl FromLeBytes for u8 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 1];
        bytes.read_exact(&mut buf).unwrap();
        buf[0]
    }
}

impl FromLeBytes for u16 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 2];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_u16(&buf)
    }
}

impl FromLeBytes for u32 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 4];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_u32(&buf)
    }
}

impl FromLeBytes for u64 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 8];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_u64(&buf)
    }
}

impl FromLeBytes for i8 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 1];
        bytes.read_exact(&mut buf).unwrap();
        i8::from_le_bytes(buf)
    }
}

impl FromLeBytes for i16 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 2];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_i16(&buf)
    }
}

impl FromLeBytes for i32 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 4];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_i32(&buf)
    }
}

impl FromLeBytes for i64 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self 
    where T: Read{
        let mut buf = [0u8; 8];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_i64(&buf)
    }
}

impl FromLeBytes for f32 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self
    where T: Read {
        let mut buf = [0u8; 4];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_f32(&buf)
    }
}

impl FromLeBytes for f64 {
    fn from_le_bytes<T>(bytes: &mut T) -> Self
    where T: Read {
        let mut buf = [0u8; 8];
        bytes.read_exact(&mut buf).unwrap();
        LittleEndian::read_f64(&buf)
    }
}

pub fn parse_le_value<T>(cur: &mut Cursor<Vec<u8>>) -> T
    where T: FromLeBytes {
        T::from_le_bytes(cur)
    }


pub fn parse_be_value<T>(cur: &mut Cursor<Vec<u8>>) -> T
    where T: FromBeBytes {
        T::from_be_bytes(cur)
    }



#[cfg(test)]
pub mod serde_tests {
    use super::*;
    use rstest::*;
    use std::io::Cursor;

    #[rstest]
    fn test_u8_from_le_bytes() {
        let mut cursor = Cursor::new(vec![0x12u8]);
        assert_eq!(0x12u8, parse_le_value(&mut cursor));
    }

    #[rstest]
    fn test_u16_from_le_bytes() {
        let mut cursor = Cursor::new(vec![0x12u8, 0x34]);
        assert_eq!(0x3412u16, parse_le_value(&mut cursor));
    }

    #[rstest]
    fn test_f32_from_le_bytes() {
        let mut cursor = Cursor::new(vec![0x00u8, 0x00, 0x48, 0x41]);
        assert_eq!(12.5f32, parse_le_value(&mut cursor));
    }

    #[rstest]
    fn test_f32_from_be_bytes() {
        let mut cursor = Cursor::new(vec![0x41u8, 0x48, 0x00, 0x00]);
        assert_eq!(12.5f32, parse_be_value(&mut cursor));
    }

    #[rstest]
    fn test_f64_from_be_bytes() {
        let mut cursor = Cursor::new(vec![0x41u8, 0x48, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(3145728.0f64, parse_be_value(&mut cursor));
    }
}