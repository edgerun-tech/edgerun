## Qwen Added Memories
- Multiple AI agents work concurrently on this repo. NEVER use destructive git commands (git clean, git reset --hard, git stash on files you didn't create, git checkout -- .) as they destroy other agents' in-progress work. Only stage and commit your own changes. Always git pull --rebase before starting work. Never force-push. Never delete files you didn't create.

## Architecture Decisions (Protocol Conformance)

### No sync TLS — async only
The workspace has NO sync TLS (`TlsStream`, `TlsServerStream`). All TLS is async via `AsyncTlsStream` and `AsyncTlsServerStream` from `edgerun-tls`. This eliminates the root cause of blocking/hanging: no more blocking `TcpStream::connect_timeout` + sync TLS handshakes with 300s read timeouts.

### edgerun-oci uses edgerun_http::HttpClient
The OCI registry client uses `edgerun_http::HttpClient` (async, HTTPS via `tls` feature) instead of raw TLS + HTTP. CLI commands (`ert pull/push/run`) use `edgerun_rt::Runtime::block_on` to call async methods. This is the correct architecture: `edgerun-tls` → async only, `edgerun-http` → HTTPS via async TLS, `edgerun-oci` → async registry client.

### edgerun-core/crypto domain separation
The signing path in `edgerun_core::crypto` uses: `SHA-256(sig_domain_tag || 0x00 || SHA-256(canonical_bytes))` (double-hash). This is the correct internal path. The `command_dispatch.rs` file has its own inline `verify_command_signature`/`verify_delegation_signature` that compute `SHA-256(canonical_bytes)` directly — these are separate code paths that happen to work because both signer and verifier use the same path. For spec conformance, all signing should go through `sign_canonical_record` / `verify_canonical_record` in `edgerun_core::crypto`.

### Canonicalization uses prost::Message::encode
The spec §17 requires deterministic canonical bytes. `prost::Message::encode()` produces deterministic output for the same input struct (ascending field numbers, deterministic varint). Unknown fields are dropped and never participate in canonicalization — correct per spec.

### Blob encryption at rest
The `edgerun-storage` blob store encrypts all blobs with AES-GCM. Key derivation is from node private key (software) or hardware-sealed (TPM/YubiKey). Recipients are tracked in `.meta` sidecar files.

### Storage: file-based indexes, no SQLite
The workspace uses **no SQLite**. Indexes are binary append-only files with in-memory HashMaps (`FileIndex` in `edgerun-storage/src/file_index.rs`). All indexes are rebuildable from the event log + encrypted blobs. The protocol spec (§6.1-§6.3) has been updated to reflect this — "file-based binary indexes" replaces all previous SQLite references.

### Core/Adapter boundary
The `edgerun-core` crate knows abstract protocol types only. Concrete transports (BLE, QUIC, filesystem, UI) live in adapter crates (`edgerun-node`, `edgerun-storage`, etc.). The proto types define `TransportClass` enum which encodes concrete transport names — this is a spec-level definition, not a core violation.
