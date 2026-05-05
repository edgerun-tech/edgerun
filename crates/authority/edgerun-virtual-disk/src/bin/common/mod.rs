use edgerun_virtual_disk::BlockError;

pub fn parse_u16(value: &str, field: &str) -> Result<u16, BlockError> {
    value
        .parse::<u16>()
        .map_err(|err| BlockError::ProtocolError(format!("invalid {field}: {err}")))
}

pub fn parse_u32(value: &str, field: &str) -> Result<u32, BlockError> {
    value
        .parse::<u32>()
        .map_err(|err| BlockError::ProtocolError(format!("invalid {field}: {err}")))
}

pub fn parse_u64(value: &str, field: &str) -> Result<u64, BlockError> {
    value
        .parse::<u64>()
        .map_err(|err| BlockError::ProtocolError(format!("invalid {field}: {err}")))
}

pub fn encode_hex(bytes: &[u8]) -> String {
    edgerun_encoding::hex::bytes_to_hex(bytes)
}

pub fn decode_hex(input: &str) -> Result<Vec<u8>, BlockError> {
    edgerun_encoding::hex::hex_to_bytes(input)
        .map_err(|err| BlockError::ProtocolError(format!("invalid hex: {err}")))
}
