use alloc::vec::Vec;

use crate::byteorder::{push_u32_le, read_u16_le, read_u32_le};
use crate::crc32::crc32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionError {
    InvalidGzipHeader,
    InvalidGzipFlags,
    TruncatedGzipHeader,
    TruncatedGzipBody,
    InvalidDeflate,
    CrcMismatch,
    SizeMismatch,
    LimitExceeded,
}

pub fn deflate_raw_compress(data: &[u8], level: u8) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(data, level)
}

pub fn deflate_raw_decompress(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    miniz_oxide::inflate::decompress_to_vec(data).map_err(|_| CompressionError::InvalidDeflate)
}

pub fn deflate_raw_decompress_with_limit(
    data: &[u8],
    limit: usize,
) -> Result<Vec<u8>, CompressionError> {
    miniz_oxide::inflate::decompress_to_vec_with_limit(data, limit).map_err(|error| match error.status {
        miniz_oxide::inflate::TINFLStatus::HasMoreOutput => CompressionError::LimitExceeded,
        _ => CompressionError::InvalidDeflate,
    })
}

pub fn zlib_compress(data: &[u8], level: u8) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, level)
}

pub fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    miniz_oxide::inflate::decompress_to_vec_zlib(data).map_err(|_| CompressionError::InvalidDeflate)
}

pub fn gzip_compress(data: &[u8], level: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 18);
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff]);
    out.extend_from_slice(&deflate_raw_compress(data, level));
    push_u32_le(&mut out, crc32(data));
    push_u32_le(&mut out, data.len() as u32);
    out
}

pub fn gzip_decompress(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b || data[2] != 8 {
        return Err(CompressionError::InvalidGzipHeader);
    }

    let flags = data[3];
    if flags & 0xe0 != 0 {
        return Err(CompressionError::InvalidGzipFlags);
    }

    let mut pos = 10usize;
    if flags & 0x04 != 0 {
        if pos + 2 > data.len() {
            return Err(CompressionError::TruncatedGzipHeader);
        }
        let xlen = read_u16_le(data, pos) as usize;
        pos = pos
            .checked_add(2 + xlen)
            .ok_or(CompressionError::TruncatedGzipHeader)?;
    }
    if flags & 0x08 != 0 {
        pos = skip_zero_terminated(data, pos)?;
    }
    if flags & 0x10 != 0 {
        pos = skip_zero_terminated(data, pos)?;
    }
    if flags & 0x02 != 0 {
        pos = pos
            .checked_add(2)
            .ok_or(CompressionError::TruncatedGzipHeader)?;
    }
    if pos + 8 > data.len() {
        return Err(CompressionError::TruncatedGzipBody);
    }

    let footer = data.len() - 8;
    let out = deflate_raw_decompress(&data[pos..footer])?;
    let expected_crc = read_u32_le(data, footer);
    let expected_len = read_u32_le(data, footer + 4);
    if expected_crc != crc32(&out) {
        return Err(CompressionError::CrcMismatch);
    }
    if expected_len != out.len() as u32 {
        return Err(CompressionError::SizeMismatch);
    }
    Ok(out)
}

fn skip_zero_terminated(data: &[u8], mut pos: usize) -> Result<usize, CompressionError> {
    while pos < data.len() {
        let byte = data[pos];
        pos += 1;
        if byte == 0 {
            return Ok(pos);
        }
    }
    Err(CompressionError::TruncatedGzipHeader)
}
