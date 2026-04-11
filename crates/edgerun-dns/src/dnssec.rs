//! DNSSEC validation (RFC 4033–4035).
//!
//! Validates RRSIG signatures against DNSKEY records and verifies
//! the chain of trust from a trust anchor (DS → DNSKEY → RRSIG).

use edgerun_crypto::sha2::{Digest, Sha256, Sha384};

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

/// Compute the key tag for a DNSKEY record (RFC 4034 Appendix B).
pub fn compute_key_tag(dnskey: &DnsRecord) -> u16 {
    if let DnsRecordData::DNSKEY {
        flags, algorithm, public_key, ..
    } = &dnskey.data {
        if *algorithm == 1 {
            // Algorithm 1 (RSAMD5) uses a different calculation
            let key_bytes = public_key;
            let len = key_bytes.len();
            if len >= 3 {
                return u16::from_be_bytes([key_bytes[len - 3], key_bytes[len - 2]]);
            }
            return 0;
        }

        // For all other algorithms:
        let mut ac = u32::from(*flags & 0xFF) + u32::from(*algorithm) + u32::from(flags >> 8);
        for (i, &b) in public_key.iter().enumerate() {
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
        algorithm, expiration, inception, key_tag, signer_name, signature, ..
    } = &rrsig.data
    {
        // Check the key tag matches
        let computed_tag = compute_key_tag(dnskey);
        if computed_tag != *key_tag {
            return DnssecResult::NoKey;
        }

        // Check time validity
        let now = now.unwrap_or_else(|| std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0));
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
        type_covered, algorithm, labels, original_ttl,
        expiration, inception, key_tag, signer_name, ..
    } = &rrsig.data {
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
fn canonical_name_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_labels: Vec<_> = a_lower.split('.').filter(|l| !l.is_empty()).collect();
    let b_labels: Vec<_> = b_lower.split('.').filter(|l| !l.is_empty()).collect();

    // Compare labels from right to left (TLD first)
    let max_len = a_labels.len().max(b_labels.len());
    for i in 0..max_len {
        let a_label = a_labels.get(a_labels.len().saturating_sub(1 + i)).copied().unwrap_or("");
        let b_label = b_labels.get(b_labels.len().saturating_sub(1 + i)).copied().unwrap_or("");
        match a_label.cmp(b_label) {
            std::cmp::Ordering::Equal => continue,
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
            if signature.len() < 256 {
                return DnssecResult::BadSignature;
            }
            // RSA public key in DNSKEY RDATA format: exponent + modulus
            // DNSKEY public_key: exponent_length(1 or 3 bytes) + exponent + modulus
            if public_key.len() < 5 {
                return DnssecResult::BadSignature;
            }
            let (exp_len, exp_start) = if public_key[0] == 0 {
                // 3-byte exponent length
                let exp_len = ((public_key[1] as usize) << 8) | (public_key[2] as usize);
                (exp_len, 3)
            } else {
                // 1-byte exponent length
                (public_key[0] as usize, 1)
            };
            if exp_start + exp_len >= public_key.len() {
                return DnssecResult::BadSignature;
            }
            let modulus = &public_key[exp_start + exp_len..];
            let exponent = &public_key[exp_start..exp_start + exp_len];

            // Construct RsaPublicKey from modulus and exponent, then verify
            use edgerun_crypto::rsa::RsaPublicKey;
            use edgerun_crypto::sha2::{Digest, Sha256};

            let n = edgerun_crypto::rsa::BigUint::from_bytes_be(modulus);
            let e = edgerun_crypto::rsa::BigUint::from_bytes_be(exponent);

            if let Ok(rsa_pub) = RsaPublicKey::new(n, e) {
                let mut hasher = Sha256::new();
                hasher.update(signed_data);
                let hashed = hasher.finalize();
                if rsa_pub.verify(
                    edgerun_crypto::rsa::Pkcs1v15Sign::new::<Sha256>(),
                    &hashed,
                    signature,
                ).is_ok() {
                    return DnssecResult::Valid;
                }
            }
            DnssecResult::BadSignature
        }
        // ECDSAP256SHA256 (RFC 6605)
        13 => {
            if signature.len() != 64 {
                return DnssecResult::BadSignature;
            }
            if public_key.len() != 65 {
                return DnssecResult::BadSignature;
            }
            // Hash the signed data
            let mut hasher = Sha256::new();
            hasher.update(signed_data);
            let _hash = hasher.finalize();

            // Verify ECDSA P-256 signature using edgerun-crypto
            use edgerun_crypto::p256::ecdsa::{Signature, VerifyingKey};
            use edgerun_crypto::Verifier;
            use edgerun_crypto::p256::EncodedPoint;

            if let Ok(point) = EncodedPoint::from_bytes(public_key.as_slice()) {
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
            if signature.len() != 64 || public_key.len() != 32 {
                return DnssecResult::BadSignature;
            }
            use edgerun_crypto::Ed25519VerifyingKey;
            use edgerun_crypto::Ed25519Signature;
            use edgerun_crypto::Verifier;

            if let Ok(vk) = Ed25519VerifyingKey::try_from(public_key.as_slice()) {
                if let Ok(sig) = Ed25519Signature::from_slice(signature) {
                    return if vk.verify(signed_data, &sig).is_ok() {
                        DnssecResult::Valid
                    } else {
                        DnssecResult::BadSignature
                    };
                }
            }
            DnssecResult::BadSignature
        }
        // ED448 (RFC 8080)
        16 => {
            if signature.len() != 114 || public_key.len() != 57 {
                return DnssecResult::BadSignature;
            }
            use edgerun_crypto::Ed448VerifyingKey;
            use edgerun_crypto::Ed448Signature;

            let mut pk_bytes = [0u8; 57];
            pk_bytes.copy_from_slice(public_key);
            let mut sig_bytes = [0u8; 114];
            sig_bytes.copy_from_slice(signature);

            if let Ok(vk) = Ed448VerifyingKey::from_bytes(&pk_bytes) {
                let sig = Ed448Signature::from_bytes(&sig_bytes);
                return if vk.verify_raw(&sig, signed_data).is_ok() {
                    DnssecResult::Valid
                } else {
                    DnssecResult::BadSignature
                };
            }
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
            key_tag, algorithm, digest_type, digest,
        } = &ds.data {
            // Find the matching DNSKEY
            for dnskey in dnskey_records {
                if compute_key_tag(dnskey) != *key_tag {
                    continue;
                }
                if let DnsRecordData::DNSKEY {
                    algorithm: key_algo, public_key: _, ..
                } = &dnskey.data {
                    if *key_algo != *algorithm {
                        continue;
                    }

                    // Compute digest of the DNSKEY RDATA
                    let dnskey_rdata = dnskey.data.to_wire(dnskey.rtype);
                    let computed_digest = match digest_type {
                        1 => {
                            // SHA-1
                            let mut h = edgerun_crypto::sha1::Sha1::new();
                            h.update(&dnskey_rdata);
                            h.finalize().to_vec()
                        }
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
    let dnskeys: Vec<_> = answers.iter()
        .chain(authority.iter())
        .chain(additional.iter())
        .filter(|r| r.rtype == DnsRecordType::DNSKEY)
        .cloned()
        .collect();

    // Collect RRSIG records
    let rrsigs: Vec<_> = answers.iter()
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
            let covered: Vec<_> = answers.iter()
                .filter(|r| r.rtype.as_u16() == *type_covered)
                .cloned()
                .collect();

            if covered.is_empty() {
                continue;
            }

            // Find the signing DNSKEY
            let signing_key = dnskeys.iter().find(|k| {
                compute_key_tag(k) == {
                    if let DnsRecordData::RRSIG { key_tag, .. } = &rrsig.data { *key_tag } else { 0 }
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
