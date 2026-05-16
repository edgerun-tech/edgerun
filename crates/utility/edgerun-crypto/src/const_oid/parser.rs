//! OID string parser with `const` support.

use crate::const_oid::{Arc, Error, ObjectIdentifier, Result, encoder::Encoder};

/// Const-friendly OID string parser.
///
/// Parses an OID from the dotted string representation.
#[derive(Debug)]
pub(crate) struct Parser {
    /// Current arc in progress
    current_arc: Arc,

    /// BER/DER encoder
    encoder: Encoder,
}

impl Parser {
    /// Parse an OID from a dot-delimited string e.g. `1.2.840.113549.1.1.1`
    pub(crate) const fn parse(s: &str) -> Result<Self> {
        let bytes = s.as_bytes();

        if bytes.is_empty() {
            return Err(Error::Empty);
        }

        match bytes[0] {
            b'0'..=b'9' => Self {
                current_arc: 0,
                encoder: Encoder::new(),
            }
            .parse_bytes(bytes),
            actual => Err(Error::DigitExpected { actual }),
        }
    }

    /// Finish parsing, returning the result
    pub(crate) const fn finish(self) -> Result<ObjectIdentifier> {
        self.encoder.finish()
    }

    /// Parse the remaining bytes
    const fn parse_bytes(mut self, bytes: &[u8]) -> Result<Self> {
        match bytes {
            // TODO(tarcieri): use `?` when stable in `const fn`
            [] => match self.encoder.arc(self.current_arc) {
                Ok(encoder) => {
                    self.encoder = encoder;
                    Ok(self)
                }
                Err(err) => Err(err),
            },
            // TODO(tarcieri): checked arithmetic
            #[allow(clippy::arithmetic_side_effects)]
            [byte @ b'0'..=b'9', remaining @ ..] => {
                let digit = byte.saturating_sub(b'0');
                self.current_arc = self.current_arc * 10 + digit as Arc;
                self.parse_bytes(remaining)
            }
            [b'.', remaining @ ..] => {
                if remaining.is_empty() {
                    return Err(Error::TrailingDot);
                }

                // TODO(tarcieri): use `?` when stable in `const fn`
                match self.encoder.arc(self.current_arc) {
                    Ok(encoder) => {
                        self.encoder = encoder;
                        self.current_arc = 0;
                        self.parse_bytes(remaining)
                    }
                    Err(err) => Err(err),
                }
            }
            [byte, ..] => Err(Error::DigitExpected { actual: *byte }),
        }
    }
}

#[cfg(test)]
mod tests;
