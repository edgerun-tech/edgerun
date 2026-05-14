use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use edgerun_wire::{Archive, Deserialize, Serialize};
use edgerun_work::codec::{blake3_hash, wire_bytes, wire_from_bytes};
use edgerun_work::preimage::{HashBuilder, PreimageBuilder};
use edgerun_work::protocol::{Hash, WorkProtocolError};

use crate::VirtualFileSystem;

pub const VFS_WIRE_ABI_VERSION: u16 = 1;
pub const DEFAULT_OBJECT_PACKET_BYTES: usize = 64 * 1024;
pub const VFS_OBJECT_COMPRESSION_NONE: u16 = 0;
pub const VFS_OBJECT_COMPRESSION_DEFLATE_RAW: u16 = 1;
pub const VFS_OBJECT_SEAL_AES256_GCM: u16 = 1;

const VFS_OBJECT_DOMAIN: &[u8] = b"edgerun:v1:vfs:object";
const VFS_OBJECT_PACKET_DOMAIN: &[u8] = b"edgerun:v1:vfs:object-packet";
const VFS_FILE_REF_DOMAIN: &[u8] = b"edgerun:v1:vfs:file-ref";
const VFS_TREE_MANIFEST_DOMAIN: &[u8] = b"edgerun:v1:vfs:tree-manifest";
const VFS_OBJECT_TRANSFORM_DOMAIN: &[u8] = b"edgerun:v1:vfs:object-transform";
const VFS_OBJECT_SEAL_AAD_DOMAIN: &[u8] = b"edgerun:v1:vfs:object-seal-aad";

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct VfsObjectPacket {
    pub abi_version: u16,
    pub object_id: Hash,
    pub object_len: u64,
    pub packet_index: u32,
    pub packet_count: u32,
    pub offset: u64,
    pub payload_hash: Hash,
    pub packet_id: Hash,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct VfsFileRef {
    pub abi_version: u16,
    pub path: String,
    pub object_id: Hash,
    pub object_len: u64,
    pub file_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct VfsTreeManifest {
    pub abi_version: u16,
    pub root_hash: Hash,
    pub files: Vec<VfsFileRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct VfsObjectTransformRef {
    pub abi_version: u16,
    pub plaintext_object_id: Hash,
    pub plaintext_len: u64,
    pub transport_object_id: Hash,
    pub transport_len: u64,
    pub compression_kind: u16,
    pub seal_kind: u16,
    pub transform_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct VfsObjectSealRequest {
    pub abi_version: u16,
    pub plaintext_object_id: Hash,
    pub plaintext_len: u64,
    pub compression_kind: u16,
    pub seal_kind: u16,
    pub aad: Vec<u8>,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct VfsObjectUnsealRequest {
    pub abi_version: u16,
    pub transport_object_id: Hash,
    pub transport_len: u64,
    pub compression_kind: u16,
    pub seal_kind: u16,
    pub aad: Vec<u8>,
    pub sealed_envelope: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub enum VfsWireRecord {
    ObjectPacket(VfsObjectPacket),
    FileRef(VfsFileRef),
    ObjectTransformRef(VfsObjectTransformRef),
    ObjectSealRequest(VfsObjectSealRequest),
    ObjectUnsealRequest(VfsObjectUnsealRequest),
    TreeManifest(VfsTreeManifest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VfsPacketError {
    ZeroPacketSize,
    PacketTooLarge,
    EmptyPacketSet,
    MixedObject,
    DuplicatePacket,
    MissingPacket,
    HashMismatch,
    InvalidPath,
    InvalidShape,
    ObjectTooLarge,
    CompressFailed,
    DecompressFailed,
}

pub fn hash_hex(hash: &Hash) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(hash.len() * 2);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn vfs_object_id(bytes: &[u8]) -> Hash {
    HashBuilder::domain(VFS_OBJECT_DOMAIN).bytes(bytes).finish()
}

pub fn vfs_file_ref_hash(path: &str, object_id: &Hash, object_len: u64) -> Hash {
    HashBuilder::domain(VFS_FILE_REF_DOMAIN)
        .bytes(path.as_bytes())
        .hash(object_id)
        .u64(object_len)
        .finish()
}

pub fn object_to_packets(
    bytes: &[u8],
    max_payload_bytes: usize,
) -> Result<Vec<VfsObjectPacket>, VfsPacketError> {
    if max_payload_bytes == 0 {
        return Err(VfsPacketError::ZeroPacketSize);
    }
    let object_len = u64::try_from(bytes.len()).map_err(|_| VfsPacketError::ObjectTooLarge)?;
    let packet_count_usize = bytes.len().max(1).div_ceil(max_payload_bytes);
    let packet_count =
        u32::try_from(packet_count_usize).map_err(|_| VfsPacketError::ObjectTooLarge)?;
    let object_id = vfs_object_id(bytes);
    let mut packets = Vec::with_capacity(packet_count_usize);

    if bytes.is_empty() {
        packets.push(packet_for_chunk(
            object_id,
            object_len,
            0,
            packet_count,
            0,
            &[],
        )?);
        return Ok(packets);
    }

    for (index, chunk) in bytes.chunks(max_payload_bytes).enumerate() {
        let packet_index = u32::try_from(index).map_err(|_| VfsPacketError::ObjectTooLarge)?;
        let offset = u64::try_from(index.saturating_mul(max_payload_bytes))
            .map_err(|_| VfsPacketError::ObjectTooLarge)?;
        packets.push(packet_for_chunk(
            object_id,
            object_len,
            packet_index,
            packet_count,
            offset,
            chunk,
        )?);
    }
    Ok(packets)
}

pub fn packets_to_object(packets: &[VfsObjectPacket]) -> Result<Vec<u8>, VfsPacketError> {
    if packets.is_empty() {
        return Err(VfsPacketError::EmptyPacketSet);
    }

    let first = &packets[0];
    let packet_count =
        usize::try_from(first.packet_count).map_err(|_| VfsPacketError::InvalidShape)?;
    let object_len =
        usize::try_from(first.object_len).map_err(|_| VfsPacketError::ObjectTooLarge)?;
    if packet_count == 0 || packet_count != packets.len() {
        return Err(VfsPacketError::MissingPacket);
    }

    let mut ordered: Vec<Option<&VfsObjectPacket>> = vec![None; packet_count];
    for packet in packets {
        validate_packet(packet)?;
        if packet.abi_version != VFS_WIRE_ABI_VERSION
            || packet.object_id != first.object_id
            || packet.object_len != first.object_len
            || packet.packet_count != first.packet_count
        {
            return Err(VfsPacketError::MixedObject);
        }
        let index =
            usize::try_from(packet.packet_index).map_err(|_| VfsPacketError::InvalidShape)?;
        if index >= packet_count {
            return Err(VfsPacketError::MissingPacket);
        }
        if ordered[index].replace(packet).is_some() {
            return Err(VfsPacketError::DuplicatePacket);
        }
    }

    let mut out = Vec::with_capacity(object_len);
    for (index, packet) in ordered.into_iter().enumerate() {
        let packet = packet.ok_or(VfsPacketError::MissingPacket)?;
        let expected_offset = out.len();
        if usize::try_from(packet.offset).map_err(|_| VfsPacketError::ObjectTooLarge)?
            != expected_offset
        {
            return Err(VfsPacketError::InvalidShape);
        }
        if index + 1 < packet_count && packet.bytes.is_empty() {
            return Err(VfsPacketError::InvalidShape);
        }
        out.extend_from_slice(&packet.bytes);
    }

    if out.len() != object_len || vfs_object_id(&out) != first.object_id {
        return Err(VfsPacketError::HashMismatch);
    }
    Ok(out)
}

pub fn file_to_packets(
    path: impl Into<String>,
    bytes: &[u8],
    max_payload_bytes: usize,
) -> Result<(VfsFileRef, Vec<VfsObjectPacket>), VfsPacketError> {
    let path = normalize_vfs_path(path.into())?;
    let object_id = vfs_object_id(bytes);
    let object_len = u64::try_from(bytes.len()).map_err(|_| VfsPacketError::ObjectTooLarge)?;
    let file_hash = vfs_file_ref_hash(&path, &object_id, object_len);
    let file_ref = VfsFileRef {
        abi_version: VFS_WIRE_ABI_VERSION,
        path,
        object_id,
        object_len,
        file_hash,
    };
    Ok((file_ref, object_to_packets(bytes, max_payload_bytes)?))
}

pub fn file_ref_and_packets_to_entry(
    file_ref: &VfsFileRef,
    packets: &[VfsObjectPacket],
) -> Result<(String, Vec<u8>), VfsPacketError> {
    if file_ref.abi_version != VFS_WIRE_ABI_VERSION {
        return Err(VfsPacketError::InvalidShape);
    }
    let path = normalize_vfs_path(file_ref.path.clone())?;
    if file_ref.file_hash != vfs_file_ref_hash(&path, &file_ref.object_id, file_ref.object_len) {
        return Err(VfsPacketError::HashMismatch);
    }
    let bytes = packets_to_object(packets)?;
    if file_ref.object_id != vfs_object_id(&bytes)
        || file_ref.object_len != bytes.len() as u64
        || file_ref.object_id != packets[0].object_id
    {
        return Err(VfsPacketError::HashMismatch);
    }
    Ok((path, bytes))
}

pub fn prepare_object_seal_request(bytes: &[u8]) -> Result<VfsObjectSealRequest, VfsPacketError> {
    let plaintext_object_id = vfs_object_id(bytes);
    let plaintext_len = u64::try_from(bytes.len()).map_err(|_| VfsPacketError::ObjectTooLarge)?;
    let compressed = compress_object(bytes);
    let compression_kind = if compressed.len() < bytes.len() {
        VFS_OBJECT_COMPRESSION_DEFLATE_RAW
    } else {
        VFS_OBJECT_COMPRESSION_NONE
    };
    let payload = if compression_kind == VFS_OBJECT_COMPRESSION_DEFLATE_RAW {
        compressed
    } else {
        bytes.to_vec()
    };
    let aad = object_seal_aad(&plaintext_object_id, plaintext_len, compression_kind);
    Ok(VfsObjectSealRequest {
        abi_version: VFS_WIRE_ABI_VERSION,
        plaintext_object_id,
        plaintext_len,
        compression_kind,
        seal_kind: VFS_OBJECT_SEAL_AES256_GCM,
        aad,
        payload,
    })
}

pub fn prepare_file_seal_request(
    path: impl Into<String>,
    bytes: &[u8],
) -> Result<(VfsFileRef, VfsObjectSealRequest), VfsPacketError> {
    let path = normalize_vfs_path(path.into())?;
    let object_id = vfs_object_id(bytes);
    let object_len = u64::try_from(bytes.len()).map_err(|_| VfsPacketError::ObjectTooLarge)?;
    let file_hash = vfs_file_ref_hash(&path, &object_id, object_len);
    let file_ref = VfsFileRef {
        abi_version: VFS_WIRE_ABI_VERSION,
        path,
        object_id,
        object_len,
        file_hash,
    };
    Ok((file_ref, prepare_object_seal_request(bytes)?))
}

pub fn sealed_object_to_packets(
    request: &VfsObjectSealRequest,
    sealed_envelope: &[u8],
    max_payload_bytes: usize,
) -> Result<(VfsObjectTransformRef, Vec<VfsObjectPacket>), VfsPacketError> {
    validate_seal_request(request)?;
    let transport_object_id = vfs_object_id(sealed_envelope);
    let transport_len =
        u64::try_from(sealed_envelope.len()).map_err(|_| VfsPacketError::ObjectTooLarge)?;
    let mut transform = VfsObjectTransformRef {
        abi_version: VFS_WIRE_ABI_VERSION,
        plaintext_object_id: request.plaintext_object_id,
        plaintext_len: request.plaintext_len,
        transport_object_id,
        transport_len,
        compression_kind: request.compression_kind,
        seal_kind: request.seal_kind,
        transform_hash: [0u8; 32],
    };
    transform.transform_hash = vfs_object_transform_hash(&transform);
    Ok((
        transform,
        object_to_packets(sealed_envelope, max_payload_bytes)?,
    ))
}

pub fn prepare_unseal_object_from_packets(
    transform: &VfsObjectTransformRef,
    packets: &[VfsObjectPacket],
) -> Result<VfsObjectUnsealRequest, VfsPacketError> {
    validate_transform(transform)?;
    let transport = packets_to_object(packets)?;
    if transform.transport_object_id != vfs_object_id(&transport)
        || transform.transport_len != transport.len() as u64
    {
        return Err(VfsPacketError::HashMismatch);
    }

    let aad = object_seal_aad(
        &transform.plaintext_object_id,
        transform.plaintext_len,
        transform.compression_kind,
    );
    Ok(VfsObjectUnsealRequest {
        abi_version: VFS_WIRE_ABI_VERSION,
        transport_object_id: transform.transport_object_id,
        transport_len: transform.transport_len,
        compression_kind: transform.compression_kind,
        seal_kind: transform.seal_kind,
        aad,
        sealed_envelope: transport,
    })
}

pub fn unsealed_payload_to_object(
    transform: &VfsObjectTransformRef,
    payload: &[u8],
) -> Result<Vec<u8>, VfsPacketError> {
    validate_transform(transform)?;
    let bytes = match transform.compression_kind {
        VFS_OBJECT_COMPRESSION_NONE => payload.to_vec(),
        VFS_OBJECT_COMPRESSION_DEFLATE_RAW => {
            let limit = usize::try_from(transform.plaintext_len)
                .map_err(|_| VfsPacketError::ObjectTooLarge)?;
            edgerun_encoding::compression::deflate_raw_decompress_with_limit(payload, limit)
                .map_err(|_| VfsPacketError::DecompressFailed)?
        }
        _ => return Err(VfsPacketError::InvalidShape),
    };
    if transform.plaintext_object_id != vfs_object_id(&bytes)
        || transform.plaintext_len != bytes.len() as u64
    {
        return Err(VfsPacketError::HashMismatch);
    }
    Ok(bytes)
}

pub fn unsealed_file_payload_to_entry(
    file_ref: &VfsFileRef,
    transform: &VfsObjectTransformRef,
    payload: &[u8],
) -> Result<(String, Vec<u8>), VfsPacketError> {
    let bytes = unsealed_payload_to_object(transform, payload)?;
    file_plaintext_to_entry(file_ref, transform, bytes)
}

fn file_plaintext_to_entry(
    file_ref: &VfsFileRef,
    transform: &VfsObjectTransformRef,
    bytes: Vec<u8>,
) -> Result<(String, Vec<u8>), VfsPacketError> {
    if file_ref.abi_version != VFS_WIRE_ABI_VERSION {
        return Err(VfsPacketError::InvalidShape);
    }
    let path = normalize_vfs_path(file_ref.path.clone())?;
    if file_ref.file_hash != vfs_file_ref_hash(&path, &file_ref.object_id, file_ref.object_len) {
        return Err(VfsPacketError::HashMismatch);
    }
    if file_ref.object_id != transform.plaintext_object_id
        || file_ref.object_len != transform.plaintext_len
        || file_ref.object_id != vfs_object_id(&bytes)
        || file_ref.object_len != bytes.len() as u64
    {
        return Err(VfsPacketError::HashMismatch);
    }
    Ok((path, bytes))
}

pub fn files_to_manifest<I>(files: I) -> Result<VfsTreeManifest, VfsPacketError>
where
    I: IntoIterator<Item = VfsFileRef>,
{
    let mut files: Vec<VfsFileRef> = files.into_iter().collect();
    for file in &files {
        if file.abi_version != VFS_WIRE_ABI_VERSION {
            return Err(VfsPacketError::InvalidShape);
        }
        let path = normalize_vfs_path(file.path.clone())?;
        if file.file_hash != vfs_file_ref_hash(&path, &file.object_id, file.object_len) {
            return Err(VfsPacketError::HashMismatch);
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut builder = HashBuilder::domain(VFS_TREE_MANIFEST_DOMAIN).u64(files.len() as u64);
    for file in &files {
        builder = builder
            .bytes(file.path.as_bytes())
            .hash(&file.object_id)
            .u64(file.object_len)
            .hash(&file.file_hash);
    }
    Ok(VfsTreeManifest {
        abi_version: VFS_WIRE_ABI_VERSION,
        root_hash: builder.finish(),
        files,
    })
}

pub fn vfs_from_file_packets<I>(entries: I) -> Result<VirtualFileSystem, VfsPacketError>
where
    I: IntoIterator<Item = (VfsFileRef, Vec<VfsObjectPacket>)>,
{
    let mut materialized = Vec::new();
    for (file_ref, packets) in entries {
        materialized.push(file_ref_and_packets_to_entry(&file_ref, &packets)?);
    }
    VirtualFileSystem::from_entries(materialized).map_err(|_| VfsPacketError::InvalidPath)
}

pub fn vfs_wire_record_bytes(record: &VfsWireRecord) -> Result<Vec<u8>, WorkProtocolError> {
    wire_bytes(record)
}

pub fn vfs_wire_record_from_bytes(bytes: &[u8]) -> Result<VfsWireRecord, WorkProtocolError> {
    wire_from_bytes::<VfsWireRecord, ArchivedVfsWireRecord>(bytes)
}

fn packet_for_chunk(
    object_id: Hash,
    object_len: u64,
    packet_index: u32,
    packet_count: u32,
    offset: u64,
    bytes: &[u8],
) -> Result<VfsObjectPacket, VfsPacketError> {
    let payload_hash = blake3_hash(bytes);
    let mut packet = VfsObjectPacket {
        abi_version: VFS_WIRE_ABI_VERSION,
        object_id,
        object_len,
        packet_index,
        packet_count,
        offset,
        payload_hash,
        packet_id: [0u8; 32],
        bytes: bytes.to_vec(),
    };
    packet.packet_id = vfs_object_packet_id(&packet);
    Ok(packet)
}

fn validate_packet(packet: &VfsObjectPacket) -> Result<(), VfsPacketError> {
    if packet.abi_version != VFS_WIRE_ABI_VERSION || packet.packet_count == 0 {
        return Err(VfsPacketError::InvalidShape);
    }
    if packet.packet_index >= packet.packet_count {
        return Err(VfsPacketError::MissingPacket);
    }
    if packet.payload_hash != blake3_hash(&packet.bytes) {
        return Err(VfsPacketError::HashMismatch);
    }
    if packet.packet_id != vfs_object_packet_id(packet) {
        return Err(VfsPacketError::HashMismatch);
    }
    let end = packet
        .offset
        .checked_add(packet.bytes.len() as u64)
        .ok_or(VfsPacketError::ObjectTooLarge)?;
    if end > packet.object_len {
        return Err(VfsPacketError::InvalidShape);
    }
    Ok(())
}

fn validate_transform(transform: &VfsObjectTransformRef) -> Result<(), VfsPacketError> {
    if transform.abi_version != VFS_WIRE_ABI_VERSION
        || transform.seal_kind != VFS_OBJECT_SEAL_AES256_GCM
        || !matches!(
            transform.compression_kind,
            VFS_OBJECT_COMPRESSION_NONE | VFS_OBJECT_COMPRESSION_DEFLATE_RAW
        )
        || transform.transform_hash != vfs_object_transform_hash(transform)
    {
        return Err(VfsPacketError::InvalidShape);
    }
    Ok(())
}

fn validate_seal_request(request: &VfsObjectSealRequest) -> Result<(), VfsPacketError> {
    if request.abi_version != VFS_WIRE_ABI_VERSION
        || request.seal_kind != VFS_OBJECT_SEAL_AES256_GCM
        || !matches!(
            request.compression_kind,
            VFS_OBJECT_COMPRESSION_NONE | VFS_OBJECT_COMPRESSION_DEFLATE_RAW
        )
        || request.aad
            != object_seal_aad(
                &request.plaintext_object_id,
                request.plaintext_len,
                request.compression_kind,
            )
    {
        return Err(VfsPacketError::InvalidShape);
    }
    Ok(())
}

fn vfs_object_transform_hash(transform: &VfsObjectTransformRef) -> Hash {
    HashBuilder::domain(VFS_OBJECT_TRANSFORM_DOMAIN)
        .u16(transform.abi_version)
        .hash(&transform.plaintext_object_id)
        .u64(transform.plaintext_len)
        .hash(&transform.transport_object_id)
        .u64(transform.transport_len)
        .u16(transform.compression_kind)
        .u16(transform.seal_kind)
        .finish()
}

fn object_seal_aad(
    plaintext_object_id: &Hash,
    plaintext_len: u64,
    compression_kind: u16,
) -> Vec<u8> {
    PreimageBuilder::domain(VFS_OBJECT_SEAL_AAD_DOMAIN)
        .hash(plaintext_object_id)
        .u64(plaintext_len)
        .u16(compression_kind)
        .finish()
}

fn compress_object(bytes: &[u8]) -> Vec<u8> {
    edgerun_encoding::compression::deflate_raw_compress(bytes, 6)
}

fn vfs_object_packet_id(packet: &VfsObjectPacket) -> Hash {
    HashBuilder::domain(VFS_OBJECT_PACKET_DOMAIN)
        .u16(packet.abi_version)
        .hash(&packet.object_id)
        .u64(packet.object_len)
        .u64(packet.packet_index as u64)
        .u64(packet.packet_count as u64)
        .u64(packet.offset)
        .hash(&packet.payload_hash)
        .bytes(&packet.bytes)
        .finish()
}

fn normalize_vfs_path(path: String) -> Result<String, VfsPacketError> {
    let path = path.trim_matches('/').to_string();
    if path.is_empty()
        || path.starts_with('\\')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(VfsPacketError::InvalidPath);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_packets_roundtrip() {
        let bytes = b"hello edgerun vfs packet world".to_vec();
        let packets = object_to_packets(&bytes, 7).expect("packets");

        assert_eq!(packets.len(), 5);
        assert_eq!(packets[0].object_id, vfs_object_id(&bytes));
        assert_eq!(packets_to_object(&packets).expect("object"), bytes);
    }

    #[test]
    fn empty_object_roundtrips_as_single_packet() {
        let packets = object_to_packets(&[], 16).expect("packets");

        assert_eq!(packets.len(), 1);
        assert_eq!(
            packets_to_object(&packets).expect("object"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn tampered_packet_is_rejected() {
        let mut packets = object_to_packets(b"abcdef", 3).expect("packets");
        packets[1].bytes[0] = b'X';

        assert_eq!(
            packets_to_object(&packets),
            Err(VfsPacketError::HashMismatch)
        );
    }

    #[test]
    fn file_ref_materializes_entry() {
        let (file_ref, packets) =
            file_to_packets("src/lib.rs", b"pub fn ok() {}\n", 4).expect("file packets");
        let (path, bytes) = file_ref_and_packets_to_entry(&file_ref, &packets).expect("entry");

        assert_eq!(path, "src/lib.rs");
        assert_eq!(bytes, b"pub fn ok() {}\n");
    }

    #[test]
    fn invalid_path_is_rejected() {
        assert_eq!(
            file_to_packets("../src/lib.rs", b"", 4).map(|_| ()),
            Err(VfsPacketError::InvalidPath)
        );
    }

    #[test]
    fn manifest_sorts_paths_and_hashes_deterministically() {
        let (a, _) = file_to_packets("b.txt", b"b", 8).expect("a");
        let (b, _) = file_to_packets("a.txt", b"a", 8).expect("b");

        let one = files_to_manifest([a.clone(), b.clone()]).expect("manifest");
        let two = files_to_manifest([b, a]).expect("manifest");

        assert_eq!(one.files[0].path, "a.txt");
        assert_eq!(one.root_hash, two.root_hash);
    }

    #[test]
    fn wire_record_roundtrip_uses_edgerun_wire_codec() {
        let (_, packets) = file_to_packets("src/main.rs", b"fn main() {}\n", 5).expect("packets");
        let record = VfsWireRecord::ObjectPacket(packets[0].clone());
        let bytes = vfs_wire_record_bytes(&record).expect("wire bytes");

        assert_eq!(
            vfs_wire_record_from_bytes(&bytes).expect("wire record"),
            record
        );
    }

    #[test]
    fn seal_request_keeps_key_out_of_vfs_path() {
        let bytes = b"repeat repeat repeat repeat repeat repeat repeat repeat".to_vec();
        let (file_ref, request) =
            prepare_file_seal_request("src/lib.rs", &bytes).expect("seal request");

        assert_eq!(file_ref.object_id, request.plaintext_object_id);
        assert_eq!(
            request.aad,
            object_seal_aad(
                &file_ref.object_id,
                file_ref.object_len,
                request.compression_kind
            )
        );
        assert_ne!(request.payload, bytes);
    }

    #[test]
    fn sealed_envelope_packets_without_vfs_key_access() {
        let bytes = b"repeat repeat repeat repeat repeat repeat repeat repeat".to_vec();
        let (file_ref, request) =
            prepare_file_seal_request("src/lib.rs", &bytes).expect("seal request");
        let fake_trust_container_envelope = request.payload.clone();
        let (transform, packets) =
            sealed_object_to_packets(&request, &fake_trust_container_envelope, 9).expect("packets");
        let unseal_request =
            prepare_unseal_object_from_packets(&transform, &packets).expect("unseal request");

        assert_eq!(unseal_request.aad, request.aad);
        assert_eq!(
            unseal_request.sealed_envelope,
            fake_trust_container_envelope
        );
        let (path, unsealed) =
            unsealed_file_payload_to_entry(&file_ref, &transform, &request.payload)
                .expect("unsealed entry");

        assert_eq!(path, "src/lib.rs");
        assert_eq!(unsealed, bytes);
    }

    #[test]
    fn transform_tamper_is_rejected_before_unseal() {
        let request = prepare_object_seal_request(b"secret source").expect("seal request");
        let (mut transform, packets) =
            sealed_object_to_packets(&request, &request.payload, 4).expect("packets");
        transform.compression_kind = VFS_OBJECT_COMPRESSION_DEFLATE_RAW;

        assert_eq!(
            prepare_unseal_object_from_packets(&transform, &packets),
            Err(VfsPacketError::InvalidShape)
        );
    }

    #[test]
    fn materializes_vfs_from_file_packets() {
        let first = file_to_packets("src/lib.rs", b"lib", 2).expect("first");
        let second = file_to_packets("README.md", b"readme", 3).expect("second");

        let vfs = vfs_from_file_packets([first, second]).expect("vfs");

        assert_eq!(&**vfs.read("src/lib.rs").expect("lib"), b"lib");
        assert_eq!(&**vfs.read("README.md").expect("readme"), b"readme");
    }

    #[test]
    fn hash_hex_is_bla_ke_3_sized_hex() {
        let hex = hash_hex(&vfs_object_id(b"abc"));

        assert_eq!(hex.len(), 64);
        assert!(hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}
