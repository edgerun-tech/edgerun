use crate::HeaderMap;
use alloc::vec::Vec;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkedError {
    IncompleteBody,
    InvalidSize,
    TruncatedChunk,
    MissingChunkTerminator,
    InvalidTrailer,
}

impl fmt::Display for ChunkedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkedError::IncompleteBody => f.write_str("Incomplete chunked body"),
            ChunkedError::InvalidSize => f.write_str("Invalid chunk size"),
            ChunkedError::TruncatedChunk => f.write_str("Truncated chunk"),
            ChunkedError::MissingChunkTerminator => f.write_str("Missing CRLF after chunk"),
            ChunkedError::InvalidTrailer => f.write_str("Invalid trailer header"),
        }
    }
}

pub fn parse_body(data: &[u8]) -> Result<Vec<u8>, ChunkedError> {
    parse_body_with_trailers(data).map(|(body, _)| body)
}

pub fn parse_body_with_trailers(mut data: &[u8]) -> Result<(Vec<u8>, HeaderMap), ChunkedError> {
    let mut body = Vec::new();

    loop {
        let crlf = find_crlf(data, 0).ok_or(ChunkedError::IncompleteBody)?;
        let size_line =
            core::str::from_utf8(&data[..crlf]).map_err(|_| ChunkedError::InvalidSize)?;
        let size_text = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| ChunkedError::InvalidSize)?;

        data = &data[crlf + 2..];

        if size == 0 {
            return Ok((body, parse_trailers(data)?));
        }

        if data.len() < size {
            return Err(ChunkedError::TruncatedChunk);
        }

        body.extend_from_slice(&data[..size]);
        data = &data[size..];

        if data.get(..2) != Some(b"\r\n") {
            return Err(ChunkedError::MissingChunkTerminator);
        }
        data = &data[2..];
    }
}

fn find_crlf(data: &[u8], pos: usize) -> Option<usize> {
    data[pos..]
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|offset| pos + offset)
}

fn parse_trailers(data: &[u8]) -> Result<HeaderMap, ChunkedError> {
    let mut trailers = HeaderMap::new();
    let mut pos = 0;

    while pos < data.len() {
        let Some(line_end) = find_crlf(data, pos) else {
            break;
        };
        if line_end == pos {
            break;
        }

        let line =
            core::str::from_utf8(&data[pos..line_end]).map_err(|_| ChunkedError::InvalidTrailer)?;
        if let Some(colon) = line.find(':') {
            let name = line[..colon].trim();
            let value = line[colon + 1..].trim();
            if !name.is_empty() {
                let _ = trailers.insert(name, value);
            }
        }
        pos = line_end + 2;
    }

    Ok(trailers)
}
