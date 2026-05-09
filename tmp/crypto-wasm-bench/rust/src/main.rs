use std::time::Instant;

fn fill_input(len: usize) -> Vec<u8> {
    let mut data = vec![0u8; len];
    let mut x = 0x1234_5678_u32;
    for byte in &mut data {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *byte = x as u8;
    }
    data
}

fn bench_sha256(input: &[u8], iterations: usize) -> ([u8; 32], f64) {
    let start = Instant::now();
    let mut digest = [0u8; 32];
    for _ in 0..iterations {
        digest = edgerun_crypto::sha256(input);
    }
    let ns = start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64;
    (digest, ns)
}

fn bench_hmac_sha256(key: &[u8], input: &[u8], iterations: usize) -> ([u8; 32], f64) {
    let start = Instant::now();
    let mut digest = [0u8; 32];
    for _ in 0..iterations {
        let mac = edgerun_crypto::hmac_sha256(key, input);
        digest.copy_from_slice(&mac[..32]);
    }
    let ns = start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64;
    (digest, ns)
}

fn bench_p256_sign(key: &[u8; 32], digest: &[u8; 32], iterations: usize) -> (Vec<u8>, f64) {
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;

    let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(key.into())
        .expect("fixed P-256 key must be valid");
    let start = Instant::now();
    let mut signature = Vec::new();
    for _ in 0..iterations {
        let sig: edgerun_crypto::p256::ecdsa::Signature =
            signing_key.sign_prehash(digest).expect("sign prehash");
        signature.clear();
        signature.extend_from_slice(sig.to_der().as_bytes());
    }
    let ns = start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64;
    (signature, ns)
}

fn bench_ed25519_sign(key: &[u8; 32], input: &[u8], iterations: usize) -> ([u8; 64], f64) {
    use edgerun_crypto::Signer;

    let signing_key = edgerun_crypto::ed25519_dalek::SigningKey::from_bytes(key);
    let start = Instant::now();
    let mut signature = [0u8; 64];
    for _ in 0..iterations {
        signature = signing_key.sign(input).to_bytes();
    }
    let ns = start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64;
    (signature, ns)
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn main() {
    let bytes = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<usize>().ok())
        .unwrap_or(4096);
    let iterations = std::env::args()
        .nth(2)
        .and_then(|arg| arg.parse::<usize>().ok())
        .unwrap_or(20_000);
    let input = fill_input(bytes);
    let key = fill_input(32);
    let p256_key = [7u8; 32];
    let ed25519_key = [9u8; 32];
    let (sha_digest, sha_ns) = bench_sha256(&input, iterations);
    let (hmac_digest, hmac_ns) = bench_hmac_sha256(&key, &input, iterations);
    let (p256_signature, p256_ns) = bench_p256_sign(&p256_key, &sha_digest, iterations);
    let (ed25519_signature, ed25519_ns) = bench_ed25519_sign(&ed25519_key, &input, iterations);

    println!(
        "{{\"engine\":\"native-rust\",\"bytes\":{},\"iterations\":{},\"sha256_ns_per_iter\":{:.2},\"hmac_sha256_ns_per_iter\":{:.2},\"p256_sign_ns_per_iter\":{:.2},\"ed25519_sign_ns_per_iter\":{:.2},\"sha256\":\"{}\",\"hmac_sha256\":\"{}\",\"p256_signature\":\"{}\",\"ed25519_signature\":\"{}\"}}",
        bytes,
        iterations,
        sha_ns,
        hmac_ns,
        p256_ns,
        ed25519_ns,
        hex(&sha_digest),
        hex(&hmac_digest),
        hex(&p256_signature),
        hex(&ed25519_signature)
    );
}
