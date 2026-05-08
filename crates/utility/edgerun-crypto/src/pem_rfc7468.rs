#![allow(missing_docs)]

use alloc::{string::String, vec::Vec};
use core::fmt;

pub const BASE64_WRAP_WIDTH: usize = 64;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    CharacterEncoding,
    EncapsulatedText,
    HeaderDisallowed,
    Label,
    Length,
    Preamble,
    PreEncapsulationBoundary,
    PostEncapsulationBoundary,
    UnexpectedTypeLabel { expected: &'static str },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CharacterEncoding => f.write_str("PEM character encoding error"),
            Error::EncapsulatedText => f.write_str("PEM error in encapsulated text"),
            Error::HeaderDisallowed => f.write_str("PEM headers disallowed by RFC7468"),
            Error::Label => f.write_str("PEM type label invalid"),
            Error::Length => f.write_str("PEM length invalid"),
            Error::Preamble => f.write_str("PEM preamble contains invalid data"),
            Error::PreEncapsulationBoundary => {
                f.write_str("PEM pre-encapsulation boundary invalid")
            }
            Error::PostEncapsulationBoundary => {
                f.write_str("PEM post-encapsulation boundary invalid")
            }
            Error::UnexpectedTypeLabel { expected } => {
                write!(f, "unexpected PEM type label; expected {expected}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LineEnding {
    LF,
    CRLF,
}

impl Default for LineEnding {
    fn default() -> Self {
        Self::LF
    }
}

impl AsRef<str> for LineEnding {
    fn as_ref(&self) -> &str {
        match self {
            Self::LF => "\n",
            Self::CRLF => "\r\n",
        }
    }
}

pub trait PemLabel {
    const PEM_LABEL: &'static str;

    fn validate_pem_label(actual: &str) -> Result<()> {
        if Self::PEM_LABEL == actual {
            Ok(())
        } else {
            Err(Error::UnexpectedTypeLabel {
                expected: Self::PEM_LABEL,
            })
        }
    }
}

pub fn decode_label(pem: &[u8]) -> Result<&str> {
    parse_pem(pem).map(|(label, _)| label)
}

pub fn decode<'i, 'o>(pem: &'i [u8], out: &'o mut [u8]) -> Result<(&'i str, &'o [u8])> {
    let (label, body) = parse_pem(pem)?;
    let compact = compact_base64(body)?;
    let decoded =
        edgerun_encoding::base64::standard_decode(&compact).map_err(|_| Error::EncapsulatedText)?;

    if decoded.len() > out.len() {
        return Err(Error::Length);
    }

    out[..decoded.len()].copy_from_slice(&decoded);
    Ok((label, &out[..decoded.len()]))
}

pub fn decode_vec(pem: &[u8]) -> Result<(&str, Vec<u8>)> {
    let (label, body) = parse_pem(pem)?;
    let compact = compact_base64(body)?;
    let decoded =
        edgerun_encoding::base64::standard_decode(&compact).map_err(|_| Error::EncapsulatedText)?;
    Ok((label, decoded))
}

pub fn encoded_len(label: &str, line_ending: LineEnding, input: &[u8]) -> Result<usize> {
    encapsulated_len(label, line_ending, input.len())
}

pub fn encapsulated_len(label: &str, line_ending: LineEnding, input_len: usize) -> Result<usize> {
    let base64_len = input_len.checked_add(2).ok_or(Error::Length)? / 3 * 4;
    encapsulated_len_wrapped(label, line_ending, base64_len, BASE64_WRAP_WIDTH)
}

pub fn encapsulated_len_wrapped(
    label: &str,
    line_ending: LineEnding,
    base64_len: usize,
    line_width: usize,
) -> Result<usize> {
    validate_label(label.as_bytes())?;
    if line_width == 0 {
        return Err(Error::Length);
    }

    let eol = line_ending.as_ref().len();
    let lines = if base64_len == 0 {
        0
    } else {
        (base64_len + line_width - 1) / line_width
    };

    b"-----BEGIN "
        .len()
        .checked_add(label.len())
        .and_then(|len| len.checked_add(b"-----".len()))
        .and_then(|len| len.checked_add(eol))
        .and_then(|len| len.checked_add(base64_len))
        .and_then(|len| len.checked_add(lines * eol))
        .and_then(|len| len.checked_add(b"-----END ".len()))
        .and_then(|len| len.checked_add(label.len()))
        .and_then(|len| len.checked_add(b"-----".len()))
        .and_then(|len| len.checked_add(eol))
        .ok_or(Error::Length)
}

pub fn encode(label: &str, line_ending: LineEnding, input: &[u8], out: &mut [u8]) -> Result<usize> {
    let pem = encode_string(label, line_ending, input)?;
    if pem.len() > out.len() {
        return Err(Error::Length);
    }

    out[..pem.len()].copy_from_slice(pem.as_bytes());
    Ok(pem.len())
}

pub fn encode_string(label: &str, line_ending: LineEnding, input: &[u8]) -> Result<String> {
    validate_label(label.as_bytes())?;
    let encoded = edgerun_encoding::base64::standard_encode(input);
    let eol = line_ending.as_ref();
    let mut out = String::new();
    out.push_str("-----BEGIN ");
    out.push_str(label);
    out.push_str("-----");
    out.push_str(eol);

    for chunk in encoded.as_bytes().chunks(BASE64_WRAP_WIDTH) {
        out.push_str(core::str::from_utf8(chunk).map_err(|_| Error::CharacterEncoding)?);
        out.push_str(eol);
    }

    out.push_str("-----END ");
    out.push_str(label);
    out.push_str("-----");
    out.push_str(eol);
    Ok(out)
}

#[derive(Clone)]
pub struct Decoder<'i> {
    label: &'i str,
    der: Vec<u8>,
    pos: usize,
}

impl<'i> Decoder<'i> {
    pub fn new(pem: &'i [u8]) -> Result<Self> {
        let (label, der) = decode_vec(pem)?;
        Ok(Self { label, der, pos: 0 })
    }

    pub fn new_wrapped(pem: &'i [u8], _line_width: usize) -> Result<Self> {
        Self::new(pem)
    }

    pub fn type_label(&self) -> &'i str {
        self.label
    }

    pub fn remaining_len(&self) -> usize {
        self.der.len().saturating_sub(self.pos)
    }

    pub fn decode<'o>(&mut self, out: &'o mut [u8]) -> Result<&'o [u8]> {
        let len = out.len().min(self.remaining_len());
        out[..len].copy_from_slice(&self.der[self.pos..self.pos + len]);
        self.pos += len;
        Ok(&out[..len])
    }
}

pub struct Encoder<'l, 'o> {
    label: &'l str,
    line_ending: LineEnding,
    out: &'o mut [u8],
    input: Vec<u8>,
}

impl<'l, 'o> Encoder<'l, 'o> {
    pub fn new(type_label: &'l str, line_ending: LineEnding, out: &'o mut [u8]) -> Result<Self> {
        Self::new_wrapped(type_label, line_ending, BASE64_WRAP_WIDTH, out)
    }

    pub fn new_wrapped(
        type_label: &'l str,
        line_ending: LineEnding,
        _line_width: usize,
        out: &'o mut [u8],
    ) -> Result<Self> {
        validate_label(type_label.as_bytes())?;
        Ok(Self {
            label: type_label,
            line_ending,
            out,
            input: Vec::new(),
        })
    }

    pub fn type_label(&self) -> &'l str {
        self.label
    }

    pub fn encode(&mut self, input: &[u8]) -> Result<()> {
        self.input.extend_from_slice(input);
        Ok(())
    }

    pub fn finish(self) -> Result<usize> {
        encode(self.label, self.line_ending, &self.input, self.out)
    }
}

fn parse_pem(pem: &[u8]) -> Result<(&str, &[u8])> {
    if pem.contains(&0) {
        return Err(Error::Preamble);
    }

    let pem = core::str::from_utf8(pem).map_err(|_| Error::CharacterEncoding)?;
    let begin_pos = pem
        .find("-----BEGIN ")
        .ok_or(Error::PreEncapsulationBoundary)?;
    let after_begin = begin_pos + "-----BEGIN ".len();
    let begin_end = pem[after_begin..]
        .find("-----")
        .ok_or(Error::PreEncapsulationBoundary)?
        + after_begin;
    let label = &pem[after_begin..begin_end];
    validate_label(label.as_bytes())?;

    let body_start = begin_end + "-----".len();
    let end_marker = {
        let mut marker = String::from("-----END ");
        marker.push_str(label);
        marker.push_str("-----");
        marker
    };
    let end_pos = pem[body_start..]
        .find(&end_marker)
        .ok_or(Error::PostEncapsulationBoundary)?
        + body_start;
    Ok((label, pem[body_start..end_pos].as_bytes()))
}

fn compact_base64(body: &[u8]) -> Result<String> {
    let mut compact = String::new();
    for &byte in body {
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => {}
            b':' => return Err(Error::HeaderDisallowed),
            _ => compact.push(byte as char),
        }
    }
    Ok(compact)
}

fn validate_label(label: &[u8]) -> Result<()> {
    if label.is_empty() {
        return Err(Error::Label);
    }

    for &byte in label {
        if !(0x21..=0x7e).contains(&byte) {
            return Err(Error::Label);
        }
    }

    Ok(())
}
