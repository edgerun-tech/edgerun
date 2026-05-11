use std::sync::Once;

/// Compatibility shim for older Codex websocket callers.
///
/// The actual TLS stack behind this hook is the Edgerun-owned `edgerun-rusttls` crate.
pub fn ensure_rustls_crypto_provider() {
    ensure_edgerun_tls_provider();
}

pub fn ensure_edgerun_tls_provider() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = edgerun_rusttls::crypto::CryptoProvider::get_default();
    });
}
