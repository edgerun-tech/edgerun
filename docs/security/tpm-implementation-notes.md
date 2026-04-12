# TPM Implementation Notes

## Current Architecture (Verified Against Code)

**TSS2 ESAPI is NOT used.** The `edgerun-tpm` crate uses raw TPM wire commands communicated
directly with `/dev/tpmrm0` via `read()`/`write()` syscalls. The `Cargo.toml` has only
`edgerun-core` as a dependency — no `tss-esapi`, no `libtss2-esys`.

The `lib.rs` explicitly states: `// TSS2 ESAPI module removed -- we now use raw TPM commands via /dev/tpmrm0`

### Architecture

```
edgerund init
    ↓
TpmDevice::create_ecdsa_p256_signing_key()
    ↓ (raw TPM wire commands, hand-built)
/dev/tpmrm0 (Linux kernel resource manager)
```

### TPM Commands Implemented (Raw Wire Format)

| Command | Implementation | Notes |
|---------|---------------|-------|
| `CreatePrimary` | Raw command in `device.rs` | Hand-built wire format bytes |
| `EvictControl` | Raw command in `device.rs` | Hand-built wire format bytes |
| `ReadPublic` | Raw command in `wire/commands.rs` | Used for key attestation |
| `Sign` | Raw command in `wire/commands.rs` | ECDSA P-256 signing |
| `Hash` | Raw command in `wire/commands.rs` | TPM hashing |
| `StartAuthSession` | Raw command in `wire/commands.rs` | Policy sessions |
| `PolicyPCR`, `PolicyAuthorize`, `PolicyCommandCode` | Raw commands in `wire/commands.rs` | Policy-based authorization |
| `Startup` | Raw command in `wire/commands.rs` | TPM initialization |
| `VerifySignature` | Raw command in `wire/commands.rs` | Signature verification |

### Known Issues with Raw TPM Commands

1. **`/dev/tpmrm0` blocks `TPM2_StartAuthSession`** — The kernel resource manager
   intercepts and returns `EINVAL` for session creation during initialization. The code
   works around this by using `TPM_RS_PW` (password auth session) for `CreatePrimary`
   and `EvictControl` instead of HMAC/policy sessions.

2. **Handle virtualization** — The RM maps actual handles to virtual handles.
   `EvictControl` with a virtual handle returned `TPM_RC_HANDLE` in earlier testing.
   The current code uses persistent handles (0x81000001-0x810000FF range).

3. **Response format differences** — The RM modifies response formats (no `parameterSize`
   field in sessions). The wire parser must account for this.

4. **`TPM2_Startup` on `/dev/tpmrm0`** — Sending `Startup` first returns `TPM_RC_FAILURE`
   (0x100) because the RM has already initialized the TPM.

5. **Microsoft Pluton TPM quirks** — Tested on Pluton TPM; additional quirks may exist
   on other TPM implementations.

### Runtime Signing Path

After initialization (key created at persistent handle), runtime signing uses raw TPM
commands (`ReadPublic`, `Sign`, `FlushContext`) through `/dev/tpmrm0` with password auth
sessions. No session creation needed for these commands once the key handle is established.

### Crate Structure

- `edgerun-tpm/src/device.rs` — `TpmDevice<T>` generic over `TpmTransport` trait
- `edgerun-tpm/src/signing.rs` — `LinuxTpmSigningKey` implementing `TpmSigningKey` trait
- `edgerun-tpm/src/wire/commands.rs` — Raw TPM command builders
- `edgerun-tpm/src/wire/parse.rs` — Raw TPM response parsers
- `edgerun-tpm/src/wire/mod.rs` — Wire format types
- ~45 tests in the crate

### Dependency Chain

```
edgerun-tpm
    └── edgerun-core (digest types, domain separation tags)

edgerun-hardware-signing
    ├── edgerun-tpm (feature-gated)
    ├── edgerun-yubikey (feature-gated)
    └── edgerun-android-keystore (feature-gated)
```
