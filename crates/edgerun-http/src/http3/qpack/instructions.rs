//! QPACK instruction types (RFC 9204 Section 4)

/// QPACK encoder instruction
#[derive(Debug, Clone)]
pub enum EncoderInstruction {
    /// Insert Name Reference
    InsertNameReference {
        /// Static table index
        is_static: bool,
        /// Index
        index: u64,
        /// Value
        value: Vec<u8>,
    },
    /// Insert Literal
    InsertLiteral {
        /// Name
        name: Vec<u8>,
        /// Value
        value: Vec<u8>,
    },
    /// Set Dynamic Table Capacity
    SetDynamicTableCapacity {
        /// New capacity
        capacity: u64,
    },
    /// Duplicate
    Duplicate {
        /// Index to duplicate
        index: u64,
    },
}

impl EncoderInstruction {
    /// Encode to bytes
    pub fn encode(&self) -> Vec<u8> {
        match self {
            EncoderInstruction::InsertNameReference {
                is_static,
                index,
                value,
            } => {
                let mut output = Vec::new();
                if *is_static {
                    // 1 0 T N-----
                    output.push(0x80 | ((*index >> 8) & 0x7F) as u8);
                    if *index > 127 {
                        output.push((*index & 0xFF) as u8);
                    }
                } else {
                    // 1 0 T N-----
                    output.push(0x40 | ((*index >> 8) & 0x3F) as u8);
                    if *index > 63 {
                        output.push((*index & 0xFF) as u8);
                    }
                }
                output.push(value.len() as u8);
                output.extend_from_slice(value);
                output
            }
            EncoderInstruction::InsertLiteral { name, value } => {
                let mut output = Vec::new();
                // 0 0 1 T N----
                let name_len = name.len();
                if name_len < 16 {
                    output.push(0x10 | (name_len as u8));
                } else {
                    output.push(0x1F);
                    output.push((name_len & 0xFF) as u8);
                }
                output.extend_from_slice(name);
                output.push(value.len() as u8);
                output.extend_from_slice(value);
                output
            }
            EncoderInstruction::SetDynamicTableCapacity { capacity } => {
                let mut output = Vec::new();
                // 0 0 1 0 T N---
                output.push(0x20 | ((*capacity >> 8) & 0x1F) as u8);
                if *capacity > 31 {
                    output.push((*capacity & 0xFF) as u8);
                }
                output
            }
            EncoderInstruction::Duplicate { index } => {
                let mut output = Vec::new();
                // 0 0 0 T N----
                output.push((*index >> 8) as u8);
                if *index > 255 {
                    output.push((*index & 0xFF) as u8);
                }
                output
            }
        }
    }
}

/// QPACK decoder instruction
#[derive(Debug, Clone)]
pub enum DecoderInstruction {
    /// Section Acknowledgement
    SectionAck {
        /// Stream ID
        stream_id: u64,
    },
    /// Stream Cancellation
    StreamCancel {
        /// Stream ID
        stream_id: u64,
    },
    /// Insert Count Increment
    InsertCountIncrement {
        /// Increment
        increment: u64,
    },
}

impl DecoderInstruction {
    /// Encode to bytes
    pub fn encode(&self) -> Vec<u8> {
        match self {
            DecoderInstruction::SectionAck { stream_id } => {
                let mut output = Vec::new();
                // 1 T S-----
                output.push(0x80 | ((*stream_id >> 8) & 0x7F) as u8);
                if *stream_id > 127 {
                    output.push((*stream_id & 0xFF) as u8);
                }
                output
            }
            DecoderInstruction::StreamCancel { stream_id } => {
                let mut output = Vec::new();
                // 0 1 T S---
                output.push(0x40 | ((*stream_id >> 8) & 0x3F) as u8);
                if *stream_id > 63 {
                    output.push((*stream_id & 0xFF) as u8);
                }
                output
            }
            DecoderInstruction::InsertCountIncrement { increment } => {
                let mut output = Vec::new();
                // 0 0 T I---
                output.push((*increment & 0x3F) as u8);
                output
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_instruction_duplicate() {
        let inst = EncoderInstruction::Duplicate { index: 5 };
        let encoded = inst.encode();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_decoder_instruction_ack() {
        let inst = DecoderInstruction::SectionAck { stream_id: 0 };
        let encoded = inst.encode();
        assert!((encoded[0] & 0x80) != 0);
    }
}
