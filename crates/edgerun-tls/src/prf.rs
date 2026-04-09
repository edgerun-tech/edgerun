//! Pseudorandom function (PRF) for TLS key derivation

/// TLS PRF implementation
pub struct TlsPrf;

impl TlsPrf {
    /// TLS 1.2 PRF
    /// PRF(secret, label, seed) = P_hash(secret, label + seed)
    ///
    /// P_hash(secret, seed) = HMAC_hash(secret, A(1) + seed) +
    ///                        HMAC_hash(secret, A(2) + seed) +
    ///                        HMAC_hash(secret, A(3) + seed) + ...
    ///
    /// A() is defined as:
    /// A(0) = seed
    /// A(i) = HMAC_hash(secret, A(i-1))
    pub fn prf(secret: &[u8], label: &[u8], seed: &[u8], length: usize) -> Vec<u8> {
        // P_hash implementation using HMAC-SHA256
        let mut result = Vec::with_capacity(length);
        let mut a = Self::hmac_sha256(secret, &[label, seed].concat());

        while result.len() < length {
            let output = Self::hmac_sha256(secret, &[&a, label, seed].concat());
            result.extend_from_slice(&output);
            a = Self::hmac_sha256(secret, &a);
        }

        result.truncate(length);
        result
    }

    /// TLS 1.3 HKDF-Extract
    /// HKDF-Extract(salt, IKM) -> PRK
    pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> Vec<u8> {
        // HKDF-Extract is just HMAC-SHA256(salt, ikm)
        Self::hmac_sha256(salt, ikm)
    }

    /// TLS 1.3 HKDF-Expand-Label
    /// HKDF-Expand-Label(Secret, Label, Context, Length)
    pub fn hkdf_expand_label(
        secret: &[u8],
        label: &str,
        context: &[u8],
        length: usize,
    ) -> Vec<u8> {
        // Construct HKDF-Expand-Label info
        let mut hkdf_label = Vec::new();

        // Length (2 bytes)
        hkdf_label.extend_from_slice(&(length as u16).to_be_bytes());

        // Label length + "tls13 " + label
        let full_label = format!("tls13 {}", label);
        hkdf_label.push(full_label.len() as u8);
        hkdf_label.extend_from_slice(full_label.as_bytes());

        // Context length + context
        hkdf_label.extend_from_slice(&(context.len() as u8).to_be_bytes());
        hkdf_label.extend_from_slice(context);

        // HKDF-Expand
        Self::hkdf_expand(secret, &hkdf_label, length)
    }

    /// TLS 1.3 HKDF-Expand
    /// HKDF-Expand(PRK, info, Length) -> OKM
    pub fn hkdf_expand(prk: &[u8], info: &[u8], length: usize) -> Vec<u8> {
        let hash_len = 32; // SHA-256 output
        let n = (length + hash_len - 1) / hash_len;

        let mut okm = Vec::with_capacity(length);
        let mut t = Vec::new(); // T(0) is empty

        for i in 1..=n {
            // T(i) = HMAC-Hash(PRK, T(i-1) | info | i)
            let mut input = t.clone();
            input.extend_from_slice(info);
            input.push(i as u8);

            t = Self::hmac_sha256(prk, &input);
            okm.extend_from_slice(&t);
        }

        okm.truncate(length);
        okm
    }

    /// Derive TLS 1.2 keys
    pub fn derive_keys_tls12(
        pre_master_secret: &[u8],
        client_random: &[u8],
        server_random: &[u8],
        key_length: usize,
        iv_length: usize,
    ) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        // master_secret = PRF(pre_master_secret, "master secret",
        //                     client_random + server_random)[0..48]
        let master_secret = Self::prf(
            pre_master_secret,
            b"master secret",
            &[client_random, server_random].concat(),
            48,
        );

        // key_block = PRF(master_secret, "key expansion",
        //                 server_random + client_random)
        let key_block = Self::prf(
            &master_secret,
            b"key expansion",
            &[server_random, client_random].concat(),
            2 * (key_length + iv_length),
        );

        // Split key_block
        let client_write_key = key_block[0..key_length].to_vec();
        let server_write_key = key_block[key_length..2 * key_length].to_vec();
        let client_write_iv = key_block[2 * key_length..2 * key_length + iv_length].to_vec();
        let server_write_iv =
            key_block[2 * key_length + iv_length..2 * (key_length + iv_length)].to_vec();

        (
            client_write_key,
            server_write_key,
            client_write_iv,
            server_write_iv,
        )
    }

    /// Derive TLS 1.3 keys
    pub fn derive_keys_tls13(
        shared_secret: &[u8],
        client_hello_hash: &[u8],
        _server_hello_hash: &[u8],
    ) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        // Early secret
        let salt_early = vec![0u8; 32]; // Empty salt
        let early_secret = Self::hkdf_extract(&salt_early, &[0u8; 32]); // Zero IKM

        // Handshake secret
        let salt_handshake = shared_secret;
        let handshake_secret = Self::hkdf_extract(salt_handshake, &early_secret);

        // Client handshake traffic secret
        let _client_handshake_secret = Self::hkdf_expand_label(
            &handshake_secret,
            "c hs traffic",
            client_hello_hash,
            32,
        );

        // Server handshake traffic secret
        let _server_handshake_secret = Self::hkdf_expand_label(
            &handshake_secret,
            "s hs traffic",
            client_hello_hash,
            32,
        );

        // Application secret (master secret)
        let master_secret = Self::hkdf_expand_label(&handshake_secret, "master", &[], 32);

        // Client application traffic secret
        let client_traffic_secret = Self::hkdf_expand_label(
            &master_secret,
            "c ap traffic",
            &[],
            32,
        );

        // Server application traffic secret
        let server_traffic_secret = Self::hkdf_expand_label(
            &master_secret,
            "s ap traffic",
            &[],
            32,
        );

        // Derive actual keys from secrets
        let client_write_key = Self::hkdf_expand_label(
            &client_traffic_secret,
            "key",
            &[],
            16, // AES-128
        );

        let server_write_key = Self::hkdf_expand_label(
            &server_traffic_secret,
            "key",
            &[],
            16,
        );

        let client_write_iv = Self::hkdf_expand_label(
            &client_traffic_secret,
            "iv",
            &[],
            12, // 96-bit IV
        );

        let server_write_iv = Self::hkdf_expand_label(&server_traffic_secret, "iv", &[], 12);

        (
            client_write_key,
            server_write_key,
            client_write_iv,
            server_write_iv,
        )
    }

    /// HMAC-SHA256 implementation (simplified)
    /// In production, use the edgerun-hmac crate from workspace
    fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
        // HMAC(K, m) = H((K' ⊕ opad) || H((K' ⊕ ipad) || m))
        // where K' is the key padded to block size

        let block_size = 64; // SHA-256 block size
        let hash_size = 32; // SHA-256 output size

        // If key is longer than block size, hash it
        let mut key_padded = if key.len() > block_size {
            // In full implementation: SHA256(key)
            vec![0u8; hash_size]
        } else {
            key.to_vec()
        };

        // Pad key to block size
        key_padded.resize(block_size, 0);

        // Create inner and outer padded keys
        let mut key_inner = key_padded.clone();
        let mut key_outer = key_padded.clone();

        for byte in &mut key_inner {
            *byte ^= 0x36; // ipad
        }

        for byte in &mut key_outer {
            *byte ^= 0x5c; // opad
        }

        // Inner hash: H((K' ⊕ ipad) || m)
        let mut inner_msg = key_inner;
        inner_msg.extend_from_slice(message);
        let inner_hash = Self::sha256(&inner_msg);

        // Outer hash: H((K' ⊕ opad) || inner_hash)
        let mut outer_msg = key_outer;
        outer_msg.extend_from_slice(&inner_hash);
        Self::sha256(&outer_msg)
    }

    /// SHA-256 hash using the workspace sha2 crate.
    fn sha256(data: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        Sha256::digest(data).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prf_output_length() {
        let secret = b"test_secret";
        let label = b"test_label";
        let seed = b"test_seed";

        let output_16 = TlsPrf::prf(secret, label, seed, 16);
        assert_eq!(output_16.len(), 16);

        let output_32 = TlsPrf::prf(secret, label, seed, 32);
        assert_eq!(output_32.len(), 32);

        let output_48 = TlsPrf::prf(secret, label, seed, 48);
        assert_eq!(output_48.len(), 48);
    }

    #[test]
    fn test_prf_deterministic() {
        let secret = b"secret";
        let label = b"label";
        let seed = b"seed";

        let output1 = TlsPrf::prf(secret, label, seed, 32);
        let output2 = TlsPrf::prf(secret, label, seed, 32);

        assert_eq!(output1, output2);
    }

    #[test]
    fn test_hkdf_extract() {
        let salt = b"salt";
        let ikm = b"input_keying_material";

        let prk = TlsPrf::hkdf_extract(salt, ikm);
        assert_eq!(prk.len(), 32); // SHA-256 output
    }

    #[test]
    fn test_hkdf_expand_label() {
        let secret = b"secret";
        let label = "key";
        let context = b"";

        let output = TlsPrf::hkdf_expand_label(secret, label, context, 16);
        assert_eq!(output.len(), 16);
    }

    #[test]
    fn test_derive_keys_tls12() {
        let pre_master = vec![0u8; 48];
        let client_random = [1u8; 32];
        let server_random = [2u8; 32];

        let (ck, sk, civ, siv) = TlsPrf::derive_keys_tls12(
            &pre_master,
            &client_random,
            &server_random,
            16, // 128-bit key
            12, // 96-bit IV
        );

        assert_eq!(ck.len(), 16);
        assert_eq!(sk.len(), 16);
        assert_eq!(civ.len(), 12);
        assert_eq!(siv.len(), 12);
    }

    #[test]
    fn test_derive_keys_tls13() {
        let shared_secret = vec![0u8; 32];
        let ch_hash = [1u8; 32];
        let sh_hash = [2u8; 32];

        let (ck, sk, civ, siv) =
            TlsPrf::derive_keys_tls13(&shared_secret, &ch_hash, &sh_hash);

        assert_eq!(ck.len(), 16);
        assert_eq!(sk.len(), 16);
        assert_eq!(civ.len(), 12);
        assert_eq!(siv.len(), 12);
    }
}
