//! Compression utilities for DZ blocks
//!
//! This module provides compression functionality for writing compressed data blocks.

use super::error::{WriteError, WriteResult};

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// Deflate compression (no transposition)
    Deflate = 0,
    /// Transpose + Deflate compression
    TransposeDeflate = 1,
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::Deflate
    }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct Compressor {
    /// Compression type
    pub compression_type: CompressionType,
    /// Compression level (1-9)
    pub level: u8,
    /// Column count for transposition (used with TransposeDeflate)
    pub column_count: Option<u32>,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            compression_type: CompressionType::Deflate,
            level: 6,
            column_count: None,
        }
    }
}

impl Compressor {
    /// Create a new compressor with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a compressor with deflate compression
    pub fn deflate() -> Self {
        Self {
            compression_type: CompressionType::Deflate,
            level: 6,
            column_count: None,
        }
    }

    /// Create a compressor with transpose + deflate compression
    pub fn transpose_deflate(column_count: u32) -> Self {
        Self {
            compression_type: CompressionType::TransposeDeflate,
            level: 6,
            column_count: Some(column_count),
        }
    }

    /// Set the compression level (1-9)
    pub fn with_level(mut self, level: u8) -> Self {
        self.level = level.clamp(1, 9);
        self
    }

    /// Compress data
    ///
    /// Returns (compressed_data, original_length)
    pub fn compress(&self, data: &[u8]) -> WriteResult<(Vec<u8>, u64)> {
        let original_len = data.len() as u64;

        match self.compression_type {
            CompressionType::Deflate => {
                let compressed = self.compress_deflate(data)?;
                Ok((compressed, original_len))
            }
            CompressionType::TransposeDeflate => {
                let column_count = self.column_count.unwrap_or_else(|| {
                    // Default column count based on data size
                    (data.len() / 100).max(1) as u32
                });
                let transposed = self.transpose(data, column_count as usize, original_len as usize);
                let compressed = self.compress_deflate(&transposed)?;
                Ok((compressed, original_len))
            }
        }
    }

    /// Compress using deflate algorithm
    fn compress_deflate(&self, data: &[u8]) -> WriteResult<Vec<u8>> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let level = match self.level {
            1 => Compression::fast(),
            9 => Compression::best(),
            _ => Compression::new(self.level as u32),
        };

        let mut encoder = ZlibEncoder::new(Vec::new(), level);
        encoder.write_all(data).map_err(|e| WriteError::CompressionError(e.to_string()))?;
        encoder.finish().map_err(|e| WriteError::CompressionError(e.to_string()))
    }

    /// Transpose data for better compression of columnar data
    fn transpose(&self, data: &[u8], column_count: usize, total_len: usize) -> Vec<u8> {
        if column_count == 0 || total_len == 0 {
            return data.to_vec();
        }

        let row_count = total_len / column_count;
        let left_bytes = total_len - row_count * column_count;

        let mut transposed = vec![0u8; total_len];

        for i in 0..row_count {
            for j in 0..column_count {
                transposed[i * column_count + j] = data[j * row_count + i];
            }
        }

        // Copy remaining bytes
        for i in 0..left_bytes {
            transposed[row_count * column_count + i] = data[column_count * row_count + i];
        }

        transposed
    }
}

/// Decompression utility (for round-trip testing)
#[derive(Debug, Clone)]
pub struct Decompressor;

impl Decompressor {
    /// Create a new decompressor
    pub fn new() -> Self {
        Self
    }

    /// Decompress data
    ///
    /// # Arguments
    /// * `data` - Compressed data
    /// * `original_len` - Expected original data length
    /// * `zip_type` - Compression type (0=Deflate, 1=Transpose+Deflate)
    /// * `column_count` - Column count for transposition (used with zip_type=1)
    pub fn decompress(
        &self,
        data: &[u8],
        original_len: u64,
        zip_type: u8,
        column_count: u32,
    ) -> WriteResult<Vec<u8>> {
        let decompressed = self.decompress_deflate(data)?;

        if zip_type == 1 {
            // Transpose back
            Ok(self.inverse_transpose(&decompressed, column_count as usize, original_len as usize))
        } else {
            Ok(decompressed)
        }
    }

    /// Decompress using deflate algorithm
    fn decompress_deflate(&self, data: &[u8]) -> WriteResult<Vec<u8>> {
        use flate2::bufread::ZlibDecoder;
        use std::io::Read;

        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| WriteError::CompressionError(e.to_string()))?;
        Ok(decompressed)
    }

    /// Inverse transpose operation
    fn inverse_transpose(&self, data: &[u8], column_count: usize, total_len: usize) -> Vec<u8> {
        if column_count == 0 || total_len == 0 {
            return data.to_vec();
        }

        let row_count = total_len / column_count;
        let left_bytes = total_len - row_count * column_count;

        let mut original = vec![0u8; total_len];

        for i in 0..row_count {
            for j in 0..column_count {
                original[j * row_count + i] = data[i * column_count + j];
            }
        }

        // Copy remaining bytes
        for i in 0..left_bytes {
            original[column_count * row_count + i] = data[row_count * column_count + i];
        }

        original
    }
}

impl Default for Decompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_deflate() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let compressor = Compressor::deflate();
        let (compressed, original_len) = compressor.compress(&data).unwrap();

        assert!(compressed.len() < data.len());
        assert_eq!(original_len, data.len() as u64);

        let decompressor = Decompressor::new();
        let decompressed = decompressor
            .decompress(&compressed, original_len, 0, 0)
            .unwrap();

        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_decompress_transpose() {
        // Create columnar data that benefits from transposition
        let mut data = vec![0u8; 1000];
        for i in 0..10 {
            for j in 0..100 {
                data[i * 100 + j] = (i * 10 + j / 10) as u8;
            }
        }

        let compressor = Compressor::transpose_deflate(100);
        let (compressed, original_len) = compressor.compress(&data).unwrap();

        let decompressor = Decompressor::new();
        let decompressed = decompressor
            .decompress(&compressed, original_len, 1, 100)
            .unwrap();

        assert_eq!(decompressed, data);
    }
}
