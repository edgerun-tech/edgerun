## Qwen Added Memories
- ALL crypto dependencies MUST flow through edgerun-crypto. Never add crypto crates directly to edgerun-tls or any other crate. edgerun-crypto is the single dependency boundary for all crypto primitives (p256, x25519, chacha20, aes-gcm, sha2, etc.). Other crates import crypto types exclusively through edgerun-crypto re-exports.
