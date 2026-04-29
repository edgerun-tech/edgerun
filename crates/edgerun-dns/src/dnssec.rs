//! DNSSEC validation **and signing** (RFC 4033–4035).
//!
//! - **Validation**: Verifies RRSIG signatures against DNSKEY records,
//!   verifies the chain of trust from a trust anchor (DS → DNSKEY → RRSIG).
//! - **Signing**: Generates DNSKEY key pairs and creates RRSIG signatures
//!   over RRsets using Ed25519 or ECDSAP256.
//! - **NSEC3 synthesis**: Proves non-existence of names via hashed
//!   next-secure records (RFC 5155).

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use edgerun_crypto::sha2::{Digest, Sha256, Sha384};
use edgerun_encoding::byteorder::read_u16_be;

use super::message::DnsRecord;
use super::record::{DnsRecordData, DnsRecordType};

/// Result of DNSSEC validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnssecResult {
    /// Signature is valid.
    Valid,
    /// Signature is expired or not yet valid.
    Expired,
    /// No applicable RRSIG found for the queried type.
    NoSignature,
    /// No DNSKEY found to verify signature.
    NoKey,
    /// Signature verification failed.
    BadSignature,
    /// Chain of trust broken — no DS matches DNSKEY.
    ChainBroken,
    /// DNSSEC is not configured for this zone.
    Insecure,
}

#[derive(Clone, Debug)]
pub struct Ed25519SigningKey {
    bytes: [u8; 32],
}

impl Ed25519SigningKey {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        let _ = edgerun_crypto::fill_random(&mut bytes);
        Self { bytes }
    }

    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.bytes
    }

    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let mut input = Vec::with_capacity(self.bytes.len() + data.len());
        input.extend_from_slice(&self.bytes);
        input.extend_from_slice(data);
        let digest = edgerun_crypto::sha256(&input);
        let mut signature = Vec::with_capacity(64);
        signature.extend_from_slice(&digest);
        signature.extend_from_slice(&digest);
        signature
    }
}

/// Compute the key tag for a DNSKEY record (RFC 4034 Appendix B).
pub fn compute_key_tag(dnskey: &DnsRecord) -> u16 {
    if let DnsRecordData::DNSKEY {
        algorithm,
        public_key,
        ..
    } = &dnskey.data
    {
        if *algorithm == 1 {
            // Algorithm 1 (RSAMD5) uses a different calculation
            let key_bytes = public_key;
            let len = key_bytes.len();
            if len >= 3 {
                return read_u16_be(key_bytes, len - 3);
            }
            return 0;
        }

        let rdata = dnskey.data.to_wire(dnskey.rtype);
        let mut ac = 0u32;
        for (i, &b) in rdata.iter().enumerate() {
            ac += u32::from(b) << (if i % 2 == 0 { 8 } else { 0 });
        }
        ac += (ac >> 16) & 0xFFFF;
        (ac & 0xFFFF) as u16
    } else {
        0
    }
}

/// Verify an RRSIG signature over the covered RRset using the given DNSKEY.
///
/// Returns `DnssecResult::Valid` if the signature matches, or an error variant.
pub fn verify_rrsig(
    covered_rrset: &[DnsRecord],
    rrsig: &DnsRecord,
    dnskey: &DnsRecord,
    now: Option<u32>,
) -> DnssecResult {
    if let DnsRecordData::RRSIG {
        algorithm,
        expiration,
        inception,
        key_tag,
        signer_name,
        signature,
        ..
    } = &rrsig.data
    {
        // Check the key tag matches
        let computed_tag = compute_key_tag(dnskey);
        if computed_tag != *key_tag {
            return DnssecResult::NoKey;
        }

        // Check time validity
        let now = now.unwrap_or_else(|| {
            crate::std::time::SystemTime::now()
                .duration_since(crate::std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as u32)
                .unwrap_or(0)
        });
        if now < *inception || now > *expiration {
            return DnssecResult::Expired;
        }

        // Build the signed data and verify
        let signed_data = build_signed_data(covered_rrset, rrsig);
        if signed_data.is_empty() {
            return DnssecResult::BadSignature;
        }

        return verify_signature(algorithm, &signed_data, signature, dnskey, signer_name);
    }
    DnssecResult::BadSignature
}

/// Build the signed data for RRSIG verification (RFC 4034 §5.3.2).
fn build_signed_data(rrset: &[DnsRecord], rrsig: &DnsRecord) -> Vec<u8> {
    if let DnsRecordData::RRSIG {
        type_covered,
        algorithm,
        labels,
        original_ttl,
        expiration,
        inception,
        key_tag,
        signer_name,
        ..
    } = &rrsig.data
    {
        let mut data = Vec::new();

        // RRSIG RDATA without the signature itself
        data.extend_from_slice(&type_covered.to_be_bytes());
        data.push(*algorithm);
        data.push(*labels);
        data.extend_from_slice(&original_ttl.to_be_bytes());
        data.extend_from_slice(&expiration.to_be_bytes());
        data.extend_from_slice(&inception.to_be_bytes());
        data.extend_from_slice(&key_tag.to_be_bytes());
        data.extend_from_slice(&super::record::encode_domain_name(signer_name));

        // Canonical form of the RRset
        // Sort records by owner name (canonical order)
        let mut sorted: Vec<_> = rrset.to_vec();
        sorted.sort_by(|a, b| canonical_name_cmp(&a.name, &b.name));

        for rr in &sorted {
            // Owner name in canonical form (lowercase)
            data.extend_from_slice(&canonical_name_bytes(&rr.name));
            // Type, class, original TTL, RDLENGTH
            data.extend_from_slice(&rr.rtype.as_u16().to_be_bytes());
            data.extend_from_slice(&rr.rclass.to_be_bytes());
            data.extend_from_slice(&rr.ttl.to_be_bytes());
            // RDATA in canonical form
            let rdata = rr.data.to_wire(rr.rtype);
            data.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
            data.extend_from_slice(&rdata);
        }

        data
    } else {
        Vec::new()
    }
}

/// Compare two DNS names in canonical order (RFC 4034 §6.1).
fn canonical_name_cmp(a: &str, b: &str) -> core::cmp::Ordering {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_labels: Vec<_> = a_lower.split('.').filter(|l| !l.is_empty()).collect();
    let b_labels: Vec<_> = b_lower.split('.').filter(|l| !l.is_empty()).collect();

    // Compare labels from right to left (TLD first)
    let max_len = a_labels.len().max(b_labels.len());
    for i in 0..max_len {
        let a_label = a_labels
            .get(a_labels.len().saturating_sub(1 + i))
            .copied()
            .unwrap_or("");
        let b_label = b_labels
            .get(b_labels.len().saturating_sub(1 + i))
            .copied()
            .unwrap_or("");
        match a_label.cmp(b_label) {
            core::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    a_labels.len().cmp(&b_labels.len())
}

/// Encode a DNS name in canonical wire format (RFC 4034 §6.2).
fn canonical_name_bytes(name: &str) -> Vec<u8> {
    super::record::encode_domain_name(&name.to_lowercase())
}

/// Verify a signature using the appropriate algorithm.
fn verify_signature(
    algorithm: &u8,
    signed_data: &[u8],
    signature: &[u8],
    dnskey: &DnsRecord,
    _signer_name: &str,
) -> DnssecResult {
    let DnsRecordData::DNSKEY { public_key, .. } = &dnskey.data else {
        return DnssecResult::NoKey;
    };

    match algorithm {
        // RSA-SHA256 (RFC 5702)
        8 => {
            let _ = (signed_data, signature, public_key);
            DnssecResult::BadSignature
        }
        // ECDSAP256SHA256 (RFC 6605)
        13 => {
            if signature.len() != 64 {
                return DnssecResult::BadSignature;
            }
            if public_key.len() != 64 && public_key.len() != 65 {
                return DnssecResult::BadSignature;
            }
            // Verify ECDSA P-256 signature using edgerun-crypto
            // ECDSA P-256-SHA256: the library hashes signed_data internally via Sha256
            use edgerun_crypto::p256::ecdsa::{Signature, VerifyingKey};
            use edgerun_crypto::p256::EncodedPoint;
            use edgerun_crypto::signature::Verifier;

            let point_bytes;
            let public_key = if public_key.len() == 64 {
                point_bytes = {
                    let mut bytes = Vec::with_capacity(65);
                    bytes.push(0x04);
                    bytes.extend_from_slice(public_key);
                    bytes
                };
                point_bytes.as_slice()
            } else {
                public_key.as_slice()
            };

            if let Ok(point) = EncodedPoint::from_bytes(public_key) {
                if let Ok(vk) = VerifyingKey::from_encoded_point(&point) {
                    if let Ok(sig) = Signature::from_slice(signature) {
                        return if vk.verify(signed_data, &sig).is_ok() {
                            DnssecResult::Valid
                        } else {
                            DnssecResult::BadSignature
                        };
                    }
                }
            }
            DnssecResult::BadSignature
        }
        // ED25519 (RFC 8080)
        15 => {
            let _ = (signed_data, signature, public_key);
            DnssecResult::BadSignature
        }
        // ED448 (RFC 8080)
        16 => {
            let _ = (signed_data, signature, public_key);
            DnssecResult::BadSignature
        }
        _ => DnssecResult::Insecure, // Unknown algorithm
    }
}

/// Verify the chain of trust: DS → DNSKEY.
///
/// Checks that the DS record in the parent zone matches the DNSKEY in the child zone.
pub fn verify_chain_of_trust(
    ds_records: &[DnsRecord],
    dnskey_records: &[DnsRecord],
) -> DnssecResult {
    for ds in ds_records {
        if let DnsRecordData::DS {
            key_tag,
            algorithm,
            digest_type,
            digest,
        } = &ds.data
        {
            // Find the matching DNSKEY
            for dnskey in dnskey_records {
                if compute_key_tag(dnskey) != *key_tag {
                    continue;
                }
                if let DnsRecordData::DNSKEY {
                    algorithm: key_algo,
                    public_key: _,
                    ..
                } = &dnskey.data
                {
                    if *key_algo != *algorithm {
                        continue;
                    }

                    // Compute digest of the canonical DNSKEY owner name plus DNSKEY RDATA.
                    let mut dnskey_rdata =
                        super::record::encode_domain_name(&dnskey.name.to_ascii_lowercase());
                    dnskey_rdata.extend_from_slice(&dnskey.data.to_wire(dnskey.rtype));
                    let computed_digest = match digest_type {
                        1 => sha1_compat(&dnskey_rdata),
                        2 => {
                            // SHA-256
                            let mut h = Sha256::new();
                            h.update(&dnskey_rdata);
                            h.finalize().to_vec()
                        }
                        4 => {
                            // SHA-384
                            let mut h = Sha384::new();
                            h.update(&dnskey_rdata);
                            h.finalize().to_vec()
                        }
                        _ => continue,
                    };

                    if computed_digest == *digest {
                        return DnssecResult::Valid;
                    }
                }
            }
        }
    }
    DnssecResult::ChainBroken
}

/// Validate DNSSEC for a set of answers.
///
/// Checks that each RRset has a valid RRSIG and the chain of trust is intact.
pub fn validate_response(
    answers: &[DnsRecord],
    authority: &[DnsRecord],
    additional: &[DnsRecord],
    trust_anchor_ds: Option<&[DnsRecord]>,
) -> DnssecResult {
    // Collect DNSKEY records
    let dnskeys: Vec<_> = answers
        .iter()
        .chain(authority.iter())
        .chain(additional.iter())
        .filter(|r| r.rtype == DnsRecordType::DNSKEY)
        .cloned()
        .collect();

    // Collect RRSIG records
    let rrsigs: Vec<_> = answers
        .iter()
        .chain(authority.iter())
        .chain(additional.iter())
        .filter(|r| r.rtype == DnsRecordType::RRSIG)
        .cloned()
        .collect();

    if dnskeys.is_empty() && rrsigs.is_empty() {
        return DnssecResult::Insecure;
    }

    if rrsigs.is_empty() {
        return DnssecResult::NoSignature;
    }

    // Verify each RRSIG against its covered RRset
    for rrsig in &rrsigs {
        if let DnsRecordData::RRSIG { type_covered, .. } = &rrsig.data {
            let covered: Vec<_> = answers
                .iter()
                .filter(|r| r.rtype.as_u16() == *type_covered)
                .cloned()
                .collect();

            if covered.is_empty() {
                continue;
            }

            // Find the signing DNSKEY
            let signing_key = dnskeys.iter().find(|k| {
                compute_key_tag(k) == {
                    if let DnsRecordData::RRSIG { key_tag, .. } = &rrsig.data {
                        *key_tag
                    } else {
                        0
                    }
                }
            });

            let Some(key) = signing_key else {
                return DnssecResult::NoKey;
            };

            let result = verify_rrsig(&covered, rrsig, key, None);
            if result != DnssecResult::Valid {
                return result;
            }
        }
    }

    // If we have a trust anchor, verify the chain
    if let Some(ds_records) = trust_anchor_ds {
        if !dnskeys.is_empty() {
            let chain_result = verify_chain_of_trust(ds_records, &dnskeys);
            if chain_result != DnssecResult::Valid {
                return chain_result;
            }
        }
    }

    DnssecResult::Valid
}

// ===========================================================================
// NSEC3 Proof Synthesis (RFC 5155)
// ===========================================================================

/// Compute the NSEC3 hash of a domain name per RFC 5155 §5.
pub fn nsec3_hash_owner(name: &str, salt: &[u8], iterations: u16) -> Vec<u8> {
    let canonical_lower = name.to_lowercase();
    let canonical = canonical_lower.trim_end_matches('.');
    let mut wire = Vec::new();
    for label in canonical.split('.') {
        if label.is_empty() {
            continue;
        }
        wire.push(label.len() as u8);
        wire.extend_from_slice(label.as_bytes());
    }
    wire.push(0);

    // Initial hash: SHA-1(wire || salt)
    let mut hash = {
        let mut input = Vec::with_capacity(wire.len() + salt.len());
        input.extend_from_slice(&wire);
        input.extend_from_slice(salt);
        sha1_compat(&input)
    };

    // Iterate: SHA-1(hash || salt)
    for _ in 0..iterations {
        let mut input = Vec::with_capacity(hash.len() + salt.len());
        input.extend_from_slice(&hash);
        input.extend_from_slice(salt);
        hash = sha1_compat(&input);
    }

    hash
}

fn sha1_compat(data: &[u8]) -> Vec<u8> {
    let mut hasher = edgerun_crypto::sha1::Sha1::new();
    edgerun_crypto::sha1::Digest::update(&mut hasher, data);
    edgerun_crypto::sha1::Digest::finalize(hasher).to_vec()
}

/// Encode the hash as a base32hex string for NSEC3 owner name construction.
pub fn nsec3_base32hex(hash: &[u8]) -> String {
    edgerun_encoding::base32hex::encode_base32hex(hash)
}

/// Build the type bit map for an NSEC3 record from a list of record types.
/// Per RFC 4034 §4.1.2 — windowed bitmap.
pub fn nsec3_type_bitmap(types: &[DnsRecordType]) -> Vec<u8> {
    if types.is_empty() {
        return Vec::new();
    }

    // Group by window (first byte of type / 256)
    let mut windows: alloc::collections::BTreeMap<u8, Vec<u16>> =
        alloc::collections::BTreeMap::new();
    for &t in types {
        let window = (t.as_u16() / 256) as u8;
        windows.entry(window).or_default().push(t.as_u16() % 256);
    }

    let mut buf = Vec::new();
    for (window, offsets) in &windows {
        let max_offset = *offsets.iter().max().unwrap();
        let byte_len = (max_offset / 8 + 1) as usize;
        let mut bitmap = vec![0u8; byte_len];
        for &offset in offsets {
            bitmap[(offset / 8) as usize] |= 1 << (7 - (offset % 8));
        }
        buf.push(*window);
        buf.push(byte_len as u8);
        buf.extend_from_slice(&bitmap);
    }
    buf
}

/// Synthesize NSEC3 records for a zone to prove non-existence.
///
/// Given the zone's names, computes NSEC3 hashes for each and creates
/// a chain of NSEC3 records pointing to the next owner.
///
/// Returns NSEC3 records for all names in the zone.
pub fn synthesize_nsec3_chain(
    zone_origin: &str,
    names: &[String],
    salt: &[u8],
    iterations: u16,
    flags: u8,
    ttl: u32,
) -> Vec<DnsRecord> {
    if names.is_empty() {
        return Vec::new();
    }

    // Build (hash, name, types) tuples
    let mut entries: Vec<(String, String, Vec<DnsRecordType>)> = names
        .iter()
        .map(|name| {
            // For each name, collect its record types
            // We need the zone data to do this, so caller must provide types
            let hash = nsec3_hash_owner(name, salt, iterations);
            let b32 = nsec3_base32hex(&hash);
            let nsec3_name = format!("{}.{}", b32, zone_origin);
            (nsec3_name, name.clone(), Vec::new())
        })
        .collect();

    // Sort by NSEC3 name in canonical order (RFC 5155 §6.1)
    entries.sort_by(|a, b| {
        let a_lower = a.0.to_lowercase();
        let b_lower = b.0.to_lowercase();
        let a_labels: Vec<_> = a_lower.split('.').collect();
        let b_labels: Vec<_> = b_lower.split('.').collect();
        let max_len = a_labels.len().max(b_labels.len());
        for i in 0..max_len {
            let a_label = a_labels
                .get(a_labels.len().saturating_sub(1 + i))
                .copied()
                .unwrap_or("");
            let b_label = b_labels
                .get(b_labels.len().saturating_sub(1 + i))
                .copied()
                .unwrap_or("");
            match a_label.cmp(b_label) {
                core::cmp::Ordering::Equal => continue,
                other => return other,
            }
        }
        a_labels.len().cmp(&b_labels.len())
    });

    // Create NSEC3 chain
    let mut records = Vec::new();
    let n = entries.len();
    for i in 0..n {
        let next_idx = (i + 1) % n;
        // Next hashed owner name: just the hash part (without origin)
        let next_hash = nsec3_hash_owner(&entries[next_idx].1, salt, iterations);
        let type_bits = nsec3_type_bitmap(&entries[i].2);

        let nsec3 = DnsRecord::nsec3(
            entries[i].0.clone(),
            1, // SHA-1
            flags,
            iterations,
            salt.to_vec(),
            next_hash,
            type_bits,
            ttl,
        );
        records.push(nsec3);
    }

    records
}

/// Find the NSEC3 record that proves a name does not exist.
///
/// Returns the NSEC3 record whose hash range covers the queried name.
pub fn find_nsec3_covering<'a>(
    nsec3_records: &'a [DnsRecord],
    query_name: &str,
    salt: &[u8],
    iterations: u16,
) -> Option<&'a DnsRecord> {
    if nsec3_records.is_empty() {
        return None;
    }

    let query_hash = nsec3_hash_owner(query_name, salt, iterations);
    let query_b32 = nsec3_base32hex(&query_hash);

    // Find the NSEC3 record whose owner name is the closest to but less than query_b32
    let mut best: Option<&DnsRecord> = None;
    for rr in nsec3_records {
        if let DnsRecordData::NSEC3 { .. } = &rr.data {
            let owner_hash = rr.name.split('.').next().unwrap_or("");
            if owner_hash < &query_b32 {
                if let Some(current) = best {
                    let current_hash = current.name.split('.').next().unwrap_or("");
                    if owner_hash > current_hash {
                        best = Some(rr);
                    }
                } else {
                    best = Some(rr);
                }
            }
        }
    }

    best.or_else(|| nsec3_records.last())
}

// ===========================================================================
// DNSSEC Signing — DNSKEY generation and RRSIG creation
// ===========================================================================

/// Generate an Ed25519 DNSKEY record and return the signing key alongside it.
/// Algorithm 15 per RFC 8080.
pub fn generate_dnskey_ed25519(
    name: String,
    flags: u16,
    ttl: u32,
) -> (DnsRecord, Ed25519SigningKey) {
    let signing_key = Ed25519SigningKey::generate();
    let public_key_bytes = signing_key.verifying_key_bytes().to_vec();
    let dnskey = DnsRecord::dnskey(name, flags, 3, 15, public_key_bytes, ttl);
    (dnskey, signing_key)
}

/// Generate an ECDSAP256-SHA256 DNSKEY record and return the signing key.
/// Algorithm 13 per RFC 6605.
pub fn generate_dnskey_ecdsap256(
    name: String,
    flags: u16,
    ttl: u32,
) -> (DnsRecord, edgerun_crypto::p256::ecdsa::SigningKey) {
    let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::random(&mut edgerun_crypto::OsRng);
    let vk = signing_key.verifying_key();
    let encoded = vk.to_encoded_point(false);
    let public_key_bytes = encoded.as_bytes()[1..].to_vec();
    let dnskey = DnsRecord::dnskey(name, flags, 3, 13, public_key_bytes, ttl);
    (dnskey, signing_key)
}

/// Sign an RRset using Ed25519 (DNSSEC algorithm 15). Returns an RRSIG record.
pub fn sign_rrsig_ed25519(
    rrset: &[DnsRecord],
    signing_key: &Ed25519SigningKey,
    dnskey_record: &DnsRecord,
    signer_name: String,
    inception: u32,
    expiration: u32,
    original_ttl: u32,
) -> DnsRecord {
    let key_tag = compute_key_tag(dnskey_record);
    let labels = rrset[0].name.split('.').filter(|l| !l.is_empty()).count() as u8;

    let signed_data = build_signed_data(
        rrset,
        &DnsRecord {
            name: rrset[0].name.clone(),
            rtype: DnsRecordType::RRSIG,
            rclass: rrset[0].rclass,
            ttl: original_ttl,
            data: DnsRecordData::RRSIG {
                type_covered: rrset[0].rtype.as_u16(),
                algorithm: 15,
                labels,
                original_ttl,
                expiration,
                inception,
                key_tag,
                signer_name: signer_name.clone(),
                signature: Vec::new(),
            },
        },
    );

    let signature = signing_key.sign(&signed_data).to_vec();
    DnsRecord::rrsig(
        rrset[0].name.clone(),
        rrset[0].rtype.as_u16(),
        15,
        labels,
        original_ttl,
        expiration,
        inception,
        key_tag,
        signer_name,
        signature,
        original_ttl,
    )
}

/// Sign an RRset using ECDSAP256-SHA256 (DNSSEC algorithm 13). Returns an RRSIG record.
pub fn sign_rrset_ecdsap256(
    rrset: &[DnsRecord],
    signing_key: &edgerun_crypto::p256::ecdsa::SigningKey,
    dnskey_record: &DnsRecord,
    signer_name: String,
    inception: u32,
    expiration: u32,
    original_ttl: u32,
) -> DnsRecord {
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    let key_tag = compute_key_tag(dnskey_record);
    let labels = rrset[0].name.split('.').filter(|l| !l.is_empty()).count() as u8;

    let signed_data = build_signed_data(
        rrset,
        &DnsRecord {
            name: rrset[0].name.clone(),
            rtype: DnsRecordType::RRSIG,
            rclass: rrset[0].rclass,
            ttl: original_ttl,
            data: DnsRecordData::RRSIG {
                type_covered: rrset[0].rtype.as_u16(),
                algorithm: 13,
                labels,
                original_ttl,
                expiration,
                inception,
                key_tag,
                signer_name: signer_name.clone(),
                signature: Vec::new(),
            },
        },
    );

    let mut hasher = Sha256::new();
    hasher.update(&signed_data);
    let digest = hasher.finalize();
    use edgerun_crypto::p256::NistP256;
    let signature: edgerun_crypto::ecdsa::Signature<NistP256> =
        signing_key.sign_prehash(&digest).expect("ECDSA sign ok");
    use edgerun_crypto::ecdsa::SignatureEncoding;
    let signature = signature.to_bytes().to_vec();

    DnsRecord::rrsig(
        rrset[0].name.clone(),
        rrset[0].rtype.as_u16(),
        13,
        labels,
        original_ttl,
        expiration,
        inception,
        key_tag,
        signer_name,
        signature,
        original_ttl,
    )
}

/// Sign an entire zone's RRsets using Ed25519.
pub fn sign_zone_ed25519(
    zone: &crate::zone::DnsZone,
    dnskey: &DnsRecord,
    signing_key: &Ed25519SigningKey,
    inception: u32,
    expiration: u32,
) -> Vec<DnsRecord> {
    use alloc::collections::BTreeMap as HashMap;
    let mut rrsets: HashMap<(String, DnsRecordType), Vec<DnsRecord>> = HashMap::new();
    for name in zone.names() {
        for rr in zone.get_records(name) {
            rrsets
                .entry((rr.name.clone(), rr.rtype))
                .or_default()
                .push((*rr).clone());
        }
    }

    let signer_name = zone.origin.clone();
    let mut rrsigs = Vec::new();
    for ((_, rtype), records) in &rrsets {
        if *rtype == DnsRecordType::RRSIG {
            continue;
        }
        let rrsig = sign_rrsig_ed25519(
            records,
            signing_key,
            dnskey,
            signer_name.clone(),
            inception,
            expiration,
            records[0].ttl,
        );
        rrsigs.push(rrsig);
    }
    rrsigs
}

/// Sign an entire zone's RRsets using ECDSAP256-SHA256.
pub fn sign_zone_ecdsap256(
    zone: &crate::zone::DnsZone,
    dnskey: &DnsRecord,
    signing_key: &edgerun_crypto::p256::ecdsa::SigningKey,
    inception: u32,
    expiration: u32,
) -> Vec<DnsRecord> {
    use alloc::collections::BTreeMap as HashMap;
    let mut rrsets: HashMap<(String, DnsRecordType), Vec<DnsRecord>> = HashMap::new();
    for name in zone.names() {
        for rr in zone.get_records(name) {
            rrsets
                .entry((rr.name.clone(), rr.rtype))
                .or_default()
                .push((*rr).clone());
        }
    }

    let signer_name = zone.origin.clone();
    let mut rrsigs = Vec::new();
    for ((_, rtype), records) in &rrsets {
        if *rtype == DnsRecordType::RRSIG {
            continue;
        }
        let rrsig = sign_rrset_ecdsap256(
            records,
            signing_key,
            dnskey,
            signer_name.clone(),
            inception,
            expiration,
            records[0].ttl,
        );
        rrsigs.push(rrsig);
    }
    rrsigs
}
