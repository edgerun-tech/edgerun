use super::{prefix_int, prefix_string};

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Integer(prefix_int::Error),
    String(prefix_string::Error),
    InvalidPrefix(u8),
    InvalidBase(isize),
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::Integer(e) => write!(f, "invalid integer: {:?}", e),
            ParseError::String(e) => write!(f, "invalid string: {:?}", e),
            ParseError::InvalidPrefix(p) => write!(f, "invalid prefix: 0x{:02x}", p),
            ParseError::InvalidBase(b) => write!(f, "invalid base: {}", b),
        }
    }
}

impl core::error::Error for ParseError {}

impl From<prefix_int::Error> for ParseError {
    fn from(e: prefix_int::Error) -> Self {
        ParseError::Integer(e)
    }
}

impl From<prefix_string::Error> for ParseError {
    fn from(e: prefix_string::Error) -> Self {
        ParseError::String(e)
    }
}
