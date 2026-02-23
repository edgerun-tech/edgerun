// SPDX-License-Identifier: GPL-2.0-only
use blake3::Hasher;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use thiserror::Error;

pub const ENC_SEG_MAGIC: u32 = u32::from_le_bytes(*b"SEGX");
pub const ENC_SEG_VERSION: u16 = 1;
pub const CHUNK_MAGIC: u32 = u32::from_le_bytes(*b"CHK0");
pub const CHUNK_VERSION: u16 = 1;
pub const XCHACHA_NONCE_LEN: usize = 24;
pub const POLY1305_TAG_LEN: usize = 16;
const SEG_HEADER_LEN: usize = 4 + 2 + 2 + 16 + 16 + 4 + 4 + 4 + 8 + 32;
const CHUNK_HEADER_FIXED_LEN: usize = 4 + 2 + 2 + 4 + 2 + 2 + 4 + 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionMode {
    PayloadOnly = 0,
    MetadataPrivate = 1,
}

impl EncryptionMode {
    fn from_u16(v: u16) -> Result<Self, EncryptionError> {
        match v {
            0 => Ok(Self::PayloadOnly),
            1 => Ok(Self::MetadataPrivate),
            _ => Err(EncryptionError::InvalidFormat("invalid encryption mode")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SegmentEncryptionConfig {
    pub store_uuid: [u8; 16],
    pub key_epoch: u32,
    pub chunk_size: usize,
    pub mode: EncryptionMode,
    pub store_key: [u8; 32],
}

impl SegmentEncryptionConfig {
    pub fn payload_only(store_uuid: [u8; 16], store_key: [u8; 32]) -> Self {
        Self {
            store_uuid,
            key_epoch: 0,
            chunk_size: 8 * 1024 * 1024,
            mode: EncryptionMode::PayloadOnly,
            store_key,
        }
    }
}

#[derive(Debug, Clone)]
struct SegmentHeader {
    flags: u16,
    store_uuid: [u8; 16],
    segment_uid: [u8; 16],
    key_epoch: u32,
    chunk_size: u32,
    chunk_count: u32,
    plaintext_len: u64,
    transport_root: [u8; 32],
}

#[derive(Debug, Clone)]
struct ChunkFrame {
    flags: u16,
    chunk_index: u32,
    nonce: [u8; XCHACHA_NONCE_LEN],
    aad: Vec<u8>,
    ciphertext: Vec<u8>,
    tag: [u8; POLY1305_TAG_LEN],
}

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("invalid encryption format: {0}")]
    InvalidFormat(&'static str),
    #[error("AEAD failure")]
    AeadFailure,
    #[error("integrity failure: {0}")]
    Integrity(&'static str),
}

fn derive_segment_key(store_key: [u8; 32], store_uuid: [u8; 16], segment_uid: [u8; 16]) -> Key {
    let hk = Hkdf::<Sha256>::new(Some(&store_uuid), &store_key);
    let mut okm = [0u8; 32];
    let mut info = Vec::with_capacity(12 + 16);
    info.extend_from_slice(b"erfs/segkey/");
    info.extend_from_slice(&segment_uid);
    hk.expand(&info, &mut okm)
        .expect("fixed-size HKDF expand cannot fail");
    *Key::from_slice(&okm)
}

fn derive_chunk_key(segment_key: &Key, chunk_index: u32) -> Key {
    let hk = Hkdf::<Sha256>::new(None, segment_key.as_slice());
    let mut okm = [0u8; 32];
    let mut info = Vec::with_capacity(15);
    info.extend_from_slice(b"erfs/chunk/");
    info.extend_from_slice(&chunk_index.to_be_bytes());
    hk.expand(&info, &mut okm)
        .expect("fixed-size HKDF expand cannot fail");
    *Key::from_slice(&okm)
}

fn build_aad(store_uuid: [u8; 16], segment_uid: [u8; 16], chunk_index: u32, flags: u16) -> Vec<u8> {
    let mut aad = Vec::with_capacity(16 + 16 + 4 + 2);
    aad.extend_from_slice(&store_uuid);
    aad.extend_from_slice(&segment_uid);
    aad.extend_from_slice(&chunk_index.to_le_bytes());
    aad.extend_from_slice(&flags.to_le_bytes());
    aad
}

fn encode_segment_header(h: &SegmentHeader) -> Vec<u8> {
    let mut out = Vec::with_capacity(SEG_HEADER_LEN);
    out.extend_from_slice(&ENC_SEG_MAGIC.to_le_bytes());
    out.extend_from_slice(&ENC_SEG_VERSION.to_le_bytes());
    out.extend_from_slice(&h.flags.to_le_bytes());
    out.extend_from_slice(&h.store_uuid);
    out.extend_from_slice(&h.segment_uid);
    out.extend_from_slice(&h.key_epoch.to_le_bytes());
    out.extend_from_slice(&h.chunk_size.to_le_bytes());
    out.extend_from_slice(&h.chunk_count.to_le_bytes());
    out.extend_from_slice(&h.plaintext_len.to_le_bytes());
    out.extend_from_slice(&h.transport_root);
    out
}

fn decode_segment_header(data: &[u8]) -> Result<SegmentHeader, EncryptionError> {
    if data.len() < SEG_HEADER_LEN {
        return Err(EncryptionError::InvalidFormat("short segment header"));
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != ENC_SEG_MAGIC {
        return Err(EncryptionError::InvalidFormat("segment magic mismatch"));
    }
    let version = u16::from_le_bytes([data[4], data[5]]);
    if version != ENC_SEG_VERSION {
        return Err(EncryptionError::InvalidFormat("segment version mismatch"));
    }
    let flags = u16::from_le_bytes([data[6], data[7]]);
    let mut store_uuid = [0u8; 16];
    store_uuid.copy_from_slice(&data[8..24]);
    let mut segment_uid = [0u8; 16];
    segment_uid.copy_from_slice(&data[24..40]);
    let key_epoch = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
    let chunk_size = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);
    let chunk_count = u32::from_le_bytes([data[48], data[49], data[50], data[51]]);
    let plaintext_len = u64::from_le_bytes([
        data[52], data[53], data[54], data[55], data[56], data[57], data[58], data[59],
    ]);
    let mut transport_root = [0u8; 32];
    transport_root.copy_from_slice(&data[60..92]);
    Ok(SegmentHeader {
        flags,
        store_uuid,
        segment_uid,
        key_epoch,
        chunk_size,
        chunk_count,
        plaintext_len,
        transport_root,
    })
}

fn parse_chunk_frame(data: &[u8], pos: &mut usize) -> Result<ChunkFrame, EncryptionError> {
    if *pos + CHUNK_HEADER_FIXED_LEN > data.len() {
        return Err(EncryptionError::InvalidFormat("short chunk header"));
    }
    let p = *pos;
    let magic = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
    if magic != CHUNK_MAGIC {
        return Err(EncryptionError::InvalidFormat("chunk magic mismatch"));
    }
    let version = u16::from_le_bytes([data[p + 4], data[p + 5]]);
    if version != CHUNK_VERSION {
        return Err(EncryptionError::InvalidFormat("chunk version mismatch"));
    }
    let flags = u16::from_le_bytes([data[p + 6], data[p + 7]]);
    let chunk_index = u32::from_le_bytes([data[p + 8], data[p + 9], data[p + 10], data[p + 11]]);
    let nonce_len = u16::from_le_bytes([data[p + 12], data[p + 13]]) as usize;
    let tag_len = u16::from_le_bytes([data[p + 14], data[p + 15]]) as usize;
    let aad_len =
        u32::from_le_bytes([data[p + 16], data[p + 17], data[p + 18], data[p + 19]]) as usize;
    let ciphertext_len = u64::from_le_bytes([
        data[p + 20],
        data[p + 21],
        data[p + 22],
        data[p + 23],
        data[p + 24],
        data[p + 25],
        data[p + 26],
        data[p + 27],
    ]) as usize;
    if nonce_len != XCHACHA_NONCE_LEN || tag_len != POLY1305_TAG_LEN {
        return Err(EncryptionError::InvalidFormat("invalid nonce/tag length"));
    }

    let payload_len = nonce_len + aad_len + ciphertext_len + tag_len + 4;
    let end = p + CHUNK_HEADER_FIXED_LEN + payload_len;
    if end > data.len() {
        return Err(EncryptionError::InvalidFormat("short chunk payload"));
    }

    let mut c = p + CHUNK_HEADER_FIXED_LEN;
    let mut nonce = [0u8; XCHACHA_NONCE_LEN];
    nonce.copy_from_slice(&data[c..c + nonce_len]);
    c += nonce_len;
    let aad = data[c..c + aad_len].to_vec();
    c += aad_len;
    let ciphertext = data[c..c + ciphertext_len].to_vec();
    c += ciphertext_len;
    let mut tag = [0u8; POLY1305_TAG_LEN];
    tag.copy_from_slice(&data[c..c + tag_len]);
    c += tag_len;
    let frame_crc = u32::from_le_bytes([data[c], data[c + 1], data[c + 2], data[c + 3]]);

    // Validate CRC on frame bytes excluding trailing crc field.
    let computed_crc = crc32fast::hash(&data[p..c]);
    if frame_crc != computed_crc {
        return Err(EncryptionError::Integrity("chunk frame crc mismatch"));
    }

    *pos = end;
    Ok(ChunkFrame {
        flags,
        chunk_index,
        nonce,
        aad,
        ciphertext,
        tag,
    })
}

pub fn encrypt_segment_bytes(
    plaintext_segment: &[u8],
    config: &SegmentEncryptionConfig,
) -> Result<Vec<u8>, EncryptionError> {
    encrypt_segment_parts(&[plaintext_segment], plaintext_segment.len(), config)
}

pub fn encrypt_segment_parts(
    plaintext_parts: &[&[u8]],
    plaintext_len: usize,
    config: &SegmentEncryptionConfig,
) -> Result<Vec<u8>, EncryptionError> {
    let chunk_size = config.chunk_size.max(4096);
    let chunk_count = plaintext_len.div_ceil(chunk_size);

    let mut segment_uid = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut segment_uid);
    let seg_key = derive_segment_key(config.store_key, config.store_uuid, segment_uid);

    let mut chunk_hashes = Vec::with_capacity(chunk_count);
    let mut out = Vec::with_capacity(SEG_HEADER_LEN + plaintext_len + (chunk_count * 128));

    let placeholder = SegmentHeader {
        flags: config.mode as u16,
        store_uuid: config.store_uuid,
        segment_uid,
        key_epoch: config.key_epoch,
        chunk_size: chunk_size as u32,
        chunk_count: chunk_count as u32,
        plaintext_len: plaintext_len as u64,
        transport_root: [0u8; 32],
    };
    out.extend_from_slice(&encode_segment_header(&placeholder));

    let mut part_idx = 0usize;
    let mut part_off = 0usize;
    let mut remaining = plaintext_len;
    let mut boundary_chunk = vec![0u8; chunk_size];

    let advance_empty_parts = |idx: &mut usize, off: &mut usize| {
        while *idx < plaintext_parts.len() {
            let cur = plaintext_parts[*idx];
            if *off < cur.len() {
                break;
            }
            *idx += 1;
            *off = 0;
        }
    };

    for i in 0..chunk_count {
        let want = std::cmp::min(chunk_size, remaining);
        advance_empty_parts(&mut part_idx, &mut part_off);
        let plain: &[u8];

        if part_idx >= plaintext_parts.len() {
            return Err(EncryptionError::InvalidFormat("plaintext parts underflow"));
        }
        let part = plaintext_parts[part_idx];
        if part_off + want <= part.len() {
            plain = &part[part_off..part_off + want];
            part_off += want;
        } else {
            let mut filled = 0usize;
            while filled < want {
                advance_empty_parts(&mut part_idx, &mut part_off);
                let cur = plaintext_parts
                    .get(part_idx)
                    .ok_or(EncryptionError::InvalidFormat("plaintext parts underflow"))?;
                let take = std::cmp::min(want - filled, cur.len() - part_off);
                boundary_chunk[filled..filled + take]
                    .copy_from_slice(&cur[part_off..part_off + take]);
                filled += take;
                part_off += take;
            }
            plain = &boundary_chunk[..want];
        }
        remaining -= want;

        let chunk_key = derive_chunk_key(&seg_key, i as u32);
        let cipher = XChaCha20Poly1305::new(&chunk_key);
        let mut nonce = [0u8; XCHACHA_NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let aad = build_aad(config.store_uuid, segment_uid, i as u32, config.mode as u16);
        let ct_with_tag = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                chacha20poly1305::aead::Payload {
                    msg: plain,
                    aad: &aad,
                },
            )
            .map_err(|_| EncryptionError::AeadFailure)?;
        if ct_with_tag.len() < POLY1305_TAG_LEN {
            return Err(EncryptionError::AeadFailure);
        }
        let split = ct_with_tag.len() - POLY1305_TAG_LEN;
        let frame_start = out.len();

        out.extend_from_slice(&CHUNK_MAGIC.to_le_bytes());
        out.extend_from_slice(&CHUNK_VERSION.to_le_bytes());
        out.extend_from_slice(&(config.mode as u16).to_le_bytes());
        out.extend_from_slice(&(i as u32).to_le_bytes());
        out.extend_from_slice(&(XCHACHA_NONCE_LEN as u16).to_le_bytes());
        out.extend_from_slice(&(POLY1305_TAG_LEN as u16).to_le_bytes());
        out.extend_from_slice(&(aad.len() as u32).to_le_bytes());
        out.extend_from_slice(&(split as u64).to_le_bytes());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&aad);
        out.extend_from_slice(&ct_with_tag);
        let frame_crc = crc32fast::hash(&out[frame_start..]);
        out.extend_from_slice(&frame_crc.to_le_bytes());

        let hash = blake3::hash(&out[frame_start..]);
        chunk_hashes.push(*hash.as_bytes());
    }

    // Reject inputs that had extra bytes beyond plaintext_len.
    advance_empty_parts(&mut part_idx, &mut part_off);
    if part_idx < plaintext_parts.len() {
        return Err(EncryptionError::InvalidFormat("plaintext parts overflow"));
    }

    let mut root_hasher = Hasher::new();
    for h in &chunk_hashes {
        root_hasher.update(h);
    }
    let transport_root = *root_hasher.finalize().as_bytes();

    let seg_header = SegmentHeader {
        flags: config.mode as u16,
        store_uuid: config.store_uuid,
        segment_uid,
        key_epoch: config.key_epoch,
        chunk_size: chunk_size as u32,
        chunk_count: chunk_count as u32,
        plaintext_len: plaintext_len as u64,
        transport_root,
    };
    out[..SEG_HEADER_LEN].copy_from_slice(&encode_segment_header(&seg_header));
    Ok(out)
}

pub fn verify_encrypted_segment_bytes(data: &[u8]) -> Result<(), EncryptionError> {
    let h = decode_segment_header(data)?;
    let mut pos = SEG_HEADER_LEN;
    let mut chunk_hashes = Vec::with_capacity(h.chunk_count as usize);
    for _ in 0..h.chunk_count {
        let start = pos;
        let _frame = parse_chunk_frame(data, &mut pos)?;
        let hash = blake3::hash(&data[start..pos]);
        chunk_hashes.push(*hash.as_bytes());
    }
    if pos != data.len() {
        return Err(EncryptionError::InvalidFormat(
            "trailing bytes in encrypted segment",
        ));
    }
    let mut hasher = Hasher::new();
    for ch in &chunk_hashes {
        hasher.update(ch);
    }
    let computed = *hasher.finalize().as_bytes();
    if computed != h.transport_root {
        return Err(EncryptionError::Integrity("transport root mismatch"));
    }
    Ok(())
}

pub fn decrypt_segment_bytes(
    data: &[u8],
    store_key: [u8; 32],
    expected_store_uuid: [u8; 16],
) -> Result<Vec<u8>, EncryptionError> {
    let h = decode_segment_header(data)?;
    if h.store_uuid != expected_store_uuid {
        return Err(EncryptionError::Integrity("store uuid mismatch"));
    }
    verify_encrypted_segment_bytes(data)?;
    let seg_key = derive_segment_key(store_key, h.store_uuid, h.segment_uid);
    let mut pos = SEG_HEADER_LEN;
    let mut out = Vec::with_capacity(h.plaintext_len as usize);
    let mode = EncryptionMode::from_u16(h.flags)?;
    for _ in 0..h.chunk_count {
        let frame = parse_chunk_frame(data, &mut pos)?;
        if frame.flags != mode as u16 {
            return Err(EncryptionError::Integrity("chunk mode mismatch"));
        }
        let expected_aad = build_aad(h.store_uuid, h.segment_uid, frame.chunk_index, mode as u16);
        if frame.aad != expected_aad {
            return Err(EncryptionError::Integrity("aad mismatch"));
        }
        let chunk_key = derive_chunk_key(&seg_key, frame.chunk_index);
        let cipher = XChaCha20Poly1305::new(&chunk_key);
        let mut ct = frame.ciphertext.clone();
        ct.extend_from_slice(&frame.tag);
        let plain = cipher
            .decrypt(
                XNonce::from_slice(&frame.nonce),
                chacha20poly1305::aead::Payload {
                    msg: &ct,
                    aad: &frame.aad,
                },
            )
            .map_err(|_| EncryptionError::AeadFailure)?;
        out.extend_from_slice(&plain);
    }
    if out.len() != h.plaintext_len as usize {
        return Err(EncryptionError::Integrity("plaintext length mismatch"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let store_uuid = [7u8; 16];
        let key = [9u8; 32];
        let cfg = SegmentEncryptionConfig {
            store_uuid,
            key_epoch: 1,
            chunk_size: 4096,
            mode: EncryptionMode::PayloadOnly,
            store_key: key,
        };
        let mut plain = vec![0u8; 20_000];
        for (i, b) in plain.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }

        let enc = encrypt_segment_bytes(&plain, &cfg).expect("encrypt");
        verify_encrypted_segment_bytes(&enc).expect("verify");
        let got = decrypt_segment_bytes(&enc, key, store_uuid).expect("decrypt");
        assert_eq!(got, plain);
    }

    #[test]
    fn test_detects_tamper() {
        let store_uuid = [1u8; 16];
        let key = [2u8; 32];
        let cfg = SegmentEncryptionConfig::payload_only(store_uuid, key);
        let plain = b"hello encrypted world".to_vec();
        let mut enc = encrypt_segment_bytes(&plain, &cfg).expect("encrypt");
        *enc.last_mut().expect("non-empty") ^= 0xFF;
        let res = verify_encrypted_segment_bytes(&enc);
        assert!(res.is_err());
    }
}
