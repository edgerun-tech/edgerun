# Tor WAT crypto slice

Source reference: local git commit `1b243c8d6a843585bc54b4366f814cf5f56615b1`, before the Tor crypto assembly was converted to `.erobj`.

Implemented in `crypto.wat`:

- `er_sha256_init(ctx) -> ctx_or_0`
- `er_sha256_update(ctx, data, len) -> ctx_or_0`
- `er_sha256_final(ctx, out) -> out_or_0`
- `er_tor_sha256(data, len, out) -> 32_or_0`
- `er_tor_hmac_sha256(key, key_len, msg, msg_len, out) -> 32_or_0`
- `er_tor_aes128_ctr(out, in, len, key, iv) -> 1_or_0`
- `er_tor_aes_ctr(out, in, len, key, iv) -> 1_or_0`

SHA-256 context layout matches `kernel/x86_64/crypto/sha256.asm`:

- `ctx + 0`: `H[0..7]`, 32 bytes
- `ctx + 32`: byte count low 32 bits
- `ctx + 36`: byte count high 32 bits
- `ctx + 40`: partial block buffer, 64 bytes
- `ctx + 104`: buffered byte count
- `SHA256_CTX_SIZE`: 108 bytes

Exported scratch offsets:

- `TOR_WAT_SHA256_W = 4096`: SHA-256 message schedule, 256 bytes
- `TOR_WAT_HMAC_KEY_BLOCK = 4608`: HMAC key block, 64 bytes
- `TOR_WAT_HMAC_INNER_DIGEST = 4672`: HMAC inner digest or long-key digest, 32 bytes
- `TOR_WAT_WORK_SHA_CTX = 4864`: one-shot SHA/HMAC context, 108 bytes
- `TOR_WAT_AES_ROUND_KEYS = 5120`: AES-128 round keys, 176 bytes
- `TOR_WAT_AES_COUNTER = 5312`: CTR counter, 16 bytes
- `TOR_WAT_AES_STREAM = 5328`: encrypted counter block, 16 bytes
- `TOR_WAT_AES_TMP = 5344`: AES row-shift scratch, 16 bytes

Notes and gaps:

- This is self-contained WAT with no imports. Callers use exported memory for inputs, outputs, and streaming SHA-256 contexts.
- SHA-256 and HMAC-SHA256 are software implementations. The pre-erobj `tor_digest.asm` used TPM-backed SHA-256; this port preserves the Tor-facing APIs while avoiding TPM imports.
- AES-128-CTR follows the `er_tor_aes_ctr` API shape from `tor_aes.asm`; `er_tor_aes_ctr` is exported as an alias of `er_tor_aes128_ctr`.
- The fixed scratch area makes one-shot SHA/HMAC and AES-CTR non-reentrant. Streaming SHA-256 is caller-context based, but compression uses shared `TOR_WAT_SHA256_W` scratch.
- AES-256-CTR, SHA-1 relay digests, Curve25519, Ed25519, ntor, onion-service descriptor crypto, and TLS/channel logic are outside this crypto slice.
