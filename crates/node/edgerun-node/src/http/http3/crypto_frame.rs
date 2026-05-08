use alloc::vec::Vec;

/// Parse a QUIC CRYPTO frame from a decrypted packet payload.
pub(crate) fn parse_crypto_frame(data: &[u8]) -> Option<(Vec<u8>, usize)> {
    if data.first().copied()? != 0x06 {
        return None;
    }

    let mut pos = 1;
    let (_offset, n) = super::varint::quic_decode_varint_at(data, pos).ok()?;
    pos += n;

    let (length, n) = super::varint::quic_decode_varint_at(data, pos).ok()?;
    pos += n;

    let end = pos.checked_add(length as usize)?;
    if end > data.len() {
        return None;
    }

    Some((data[pos..end].to_vec(), end))
}
