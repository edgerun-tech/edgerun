# TPM Implementation Notes

## Overview

This document records the technical findings from implementing native TPM key provisioning in the edgerun node daemon (`edgerund`). The goal was to initialize TPM-backed ECDSA P-256 signing keys without shelling out to `tpm2-tools`.

## The Problem

The edgerun protocol requires each node to have a hardware-backed identity key. The TPM (Trusted Platform Module) is the standard hardware security module for this purpose on x86 systems. However, implementing TPM key provisioning natively in Rust proved significantly more complex than expected due to restrictions in the Linux kernel TPM resource manager.

## Key Findings

### 1. Kernel TPM Resource Manager Blocks Session Creation

The kernel's TPM resource manager (`/dev/tpmrm0`) intercepts and rejects `TPM2_StartAuthSession` commands at the kernel level, returning `EINVAL` (os error 22).

**What we tried:**
- Raw `write()`/`read()` to `/dev/tpmrm0` — `StartAuthSession` gets blocked
- Raw `write()`/`read()` to `/dev/tpm0` (raw device) — TPM returns `TPM_RC_INITIALIZE` (needs startup)
- Sending `TPM2_Startup` first — TPM returns `TPM_RC_FAILURE` (0x100)

**Why this matters:**
Without sessions, many TPM commands cannot be authorized. `TPM2_CreatePrimary` requires at least one session for proper authorization on most TPM implementations, especially when the key has `userWithAuth` attribute set.

**Root cause:**
The kernel TPM resource manager (introduced in Linux 4.12) manages TPM resources and prevents raw session creation to avoid resource exhaustion attacks. It only allows a subset of TPM commands through.

### 2. Handle Virtualization

The TPM resource manager assigns **virtual handles** to transient objects. When `CreatePrimary` returns, the response contains:
- **actualHandle** (e.g., `0x80FFFFFF`) — The real TPM handle
- **virtualHandle** (e.g., `0x00000111`) — The RM-assigned virtual handle

All subsequent commands must use the virtual handle. The RM internally maps virtual handles to actual TPM handles.

**Gotcha:** When we tried `EvictControl` with the virtual handle `0x00000111`, the TPM returned `TPM_RC_HANDLE` (0x00000284) — the RM couldn't recognize it in the EvictControl context because the command structure differs.

### 3. Response Format Differences

The kernel RM modifies SESSIONS response formats:
- **Raw TPM response**: `parameterSize(4) + handles + parameters + authArea`
- **RM-modified response**: `actualHandle(4) + virtualHandle(4) + parameters + authArea` (no `parameterSize` field)

This broke our response parser, which expected `parameterSize` for SESSIONS-tagged responses.

**Fix:** Detect the response structure by checking if the first 4 bytes after the header look like a handle (`>= 0x80000000`) vs. a parameter size (small value).

### 4. TSS2 ESAPI is Required

After exhaustive attempts to work around the kernel RM restrictions with raw TPM commands, the only reliable solution is to use the TSS2 ESAPI library (`libtss2-esys.so`), the same library that `tpm2-tools` uses internally.

**Why TSS2 ESAPI works:**
- It uses the TSS2 TCTI (TPM Command Transmission Interface) device layer
- The TCTI layer properly handles session creation through the kernel RM
- It manages handle virtualization internally
- It correctly parses RM-modified response formats

**The tss-esapi Rust crate** provides safe Rust bindings to TSS2 ESAPI and is the cleanest way to integrate TPM functionality without shelling out to external tools.

### 5. NV Space Limitations

Each persistent TPM handle consumes NV (Non-Volatile) storage. TPMs have limited NV space, and the error `TPM_RC_NV_SPACE` (0x0000014b) indicates insufficient room for new allocations.

**Solution:** The system scans for available persistent handles before provisioning. If NV space is full, users must evict unused handles:
```bash
tpm2_getcap handles-persistent
tpm2_evictcontrol -C o -c 0x81000001
```

### 6. Microsoft Pluton TPM Quirks

This implementation was tested on a system with Microsoft Pluton TPM firmware. Pluton TPMs have additional quirks:
- More restrictive session handling
- May require specific attribute combinations for key creation
- The TSS2 ESAPI library handles these firmware differences transparently

## Architecture

```
edgerund init
  └─> TpmDevice::create_ecdsa_p256_signing_key()
       └─> tss-esapi crate (Rust bindings)
            └─> libtss2-esys.so (TSS2 Enhanced System API)
                 └─> libtss2-tctildr.so (TCTI Loader)
                      └─> libtss2-tcti-device.so (Device TCTI)
                           └─> /dev/tpmrm0 (Kernel TPM resource manager)
                                └─> TPM hardware
```

### Runtime Signing

After initialization, runtime signing operations (`TPM2_ReadPublic`, `TPM2_Sign`) use **raw TPM commands** through `/dev/tpmrm0` with password auth sessions. This works because:
- The persistent key handle is already established
- These commands don't require `StartAuthSession`
- Password sessions (`TPM_RS_PW`) are allowed by the kernel RM

This hybrid approach — TSS2 ESAPI for provisioning, raw TPM commands for runtime — gives the best of both worlds: reliable initialization and zero-dependency runtime operations.

## TPM Command Comparison

| Operation | Implementation | Why |
|-----------|---------------|-----|
| `CreatePrimary` | TSS2 ESAPI | Requires session creation, blocked by kernel RM |
| `EvictControl` | TSS2 ESAPI | Requires handle translation that TSS2 manages |
| `ReadPublic` | Raw TPM commands | Works with password auth, no session needed |
| `Sign` | Raw TPM commands | Works with password auth, no session needed |
| `FlushContext` | Raw TPM commands | Simple command, no session needed |

## Lessons Learned

1. **Don't fight the kernel TPM resource manager** — It's designed to restrict raw TPM access for security reasons. Use TSS2.

2. **Always use `execute_with_nullauth_session` for commands that require authorization** — Even with empty auth values, ESAPI needs at least one session to structure commands correctly.

3. **Test on real hardware, not just simulators** — TPM simulators (swtpm) don't enforce the same restrictions as real TPMs, especially Pluton.

4. **NV space is finite** — Scan for available handles before allocating. Clean up unused handles.

5. **The TSS2 API has a learning curve** — The Rust `tss-esapi` crate is well-documented but differs significantly from raw TPM command construction. Builder patterns and type-safe enums replace manual byte packing.

## References

- [TCG TPM 2.0 Library Specification](https://trustedcomputinggroup.org/work-groups/trusted-platform-module/)
- [TPM2-TSS Source](https://github.com/tpm2-software/tpm2-tss)
- [tss-esapi Rust Crate](https://crates.io/crates/tss-esapi)
- [Linux TPM Driver Documentation](https://www.kernel.org/doc/html/latest/security/tpm/)
