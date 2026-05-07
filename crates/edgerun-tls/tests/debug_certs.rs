use edgerun_tls::certificate::Certificate;
use edgerun_tls::certificate_gen::{
    cert_from_pem, generate_self_signed, generate_self_signed_pem, signing_key_from_pem,
};
use edgerun_tls::prf::Hasher;

#[test]
fn debug_cert_validity() {
    let cert = generate_self_signed(&["localhost"]).unwrap();
    let parsed = Certificate::from_der(&cert.cert_der).unwrap();

    eprintln!("not_before: {}", parsed.not_before);
    eprintln!("not_after: {}", parsed.not_after);
    eprintln!(
        "duration: {} seconds, {} days",
        parsed.not_after - parsed.not_before,
        (parsed.not_after - parsed.not_before) / 86400
    );

    assert!(parsed.not_before < parsed.not_after);
    assert!(parsed.is_valid_at_unix_secs(parsed.not_before));
    assert!(parsed.is_valid_at_unix_secs(parsed.not_after));
}

#[test]
fn debug_pem_parsing() {
    let (cert_pem, key_pem) = generate_self_signed_pem(&["localhost"]).unwrap();

    eprintln!("Cert PEM:\n{}", cert_pem);
    eprintln!("\nKey PEM:\n{}", key_pem);

    // Try to parse the key
    match signing_key_from_pem(&key_pem) {
        Ok(key) => eprintln!("Key parsed successfully"),
        Err(e) => eprintln!("Key parse error: {:?}", e),
    }
}

#[test]
fn debug_hkdf() {
    let secret = vec![0xAAu8; 32];
    let sha256 = Hasher::Sha256;

    // Try with different output lengths
    for len in &[16, 32, 48, 64] {
        match std::panic::catch_unwind(|| sha256.expand_label(&secret, "key", &[], *len)) {
            Ok(result) => eprintln!(
                "expand_label with len={} succeeded: {} bytes",
                len,
                result.len()
            ),
            Err(e) => eprintln!("expand_label with len={} panicked", len),
        }
    }
}
