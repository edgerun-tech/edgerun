use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

pub fn blake3_256(data: &[u8]) -> Vec<u8> {
    blake3::hash(data).as_bytes().to_vec()
}

pub fn domain_hash(domain_tag: &str, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(domain_tag.len() + 1 + payload.len());
    buf.extend_from_slice(domain_tag.as_bytes());
    buf.push(0);
    buf.extend_from_slice(payload);
    blake3_256(&buf)
}

pub fn signature_input(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    let mut input = Vec::with_capacity(sig_domain_tag.len() + 1 + record_hash.len());
    input.extend_from_slice(sig_domain_tag.as_bytes());
    input.push(0);
    input.extend_from_slice(record_hash);
    input
}

pub fn sign_record(private_key: &SigningKey, sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    private_key
        .sign(&signature_input(sig_domain_tag, record_hash))
        .to_bytes()
        .to_vec()
}

pub fn verify_record(
    public_key: &VerifyingKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
    signature: &[u8],
) -> bool {
    let Ok(sig) = Signature::from_slice(signature) else {
        return false;
    };
    public_key
        .verify_strict(&signature_input(sig_domain_tag, record_hash), &sig)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SecretKey;

    fn test_signing_key() -> SigningKey {
        let secret: SecretKey = [7u8; 32];
        SigningKey::from_bytes(&secret)
    }

    #[test]
    fn domain_hash_changes_with_domain() {
        let a = domain_hash("lifegraph:v0:a", b"payload");
        let b = domain_hash("lifegraph:v0:b", b"payload");
        assert_ne!(a, b);
    }

    #[test]
    fn signature_input_is_domain_separated() {
        let record_hash = vec![1; 32];
        let a = signature_input("lifegraph:v0:sig:a", &record_hash);
        let b = signature_input("lifegraph:v0:sig:b", &record_hash);
        assert_ne!(a, b);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let signing = test_signing_key();
        let verifying = signing.verifying_key();
        let record_hash = vec![2; 32];
        let sig = sign_record(&signing, "lifegraph:v0:sig:test", &record_hash);
        assert!(verify_record(
            &verifying,
            "lifegraph:v0:sig:test",
            &record_hash,
            &sig
        ));
    }

    #[test]
    fn verify_rejects_wrong_domain() {
        let signing = test_signing_key();
        let verifying = signing.verifying_key();
        let record_hash = vec![3; 32];
        let sig = sign_record(&signing, "lifegraph:v0:sig:test-a", &record_hash);
        assert!(!verify_record(
            &verifying,
            "lifegraph:v0:sig:test-b",
            &record_hash,
            &sig
        ));
    }
}
