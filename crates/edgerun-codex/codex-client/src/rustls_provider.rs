use std::sync::Once;

pub fn ensure_rustls_crypto_provider() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = edgerun_rusttls::crypto::CryptoProvider::get_default();
    });
}
