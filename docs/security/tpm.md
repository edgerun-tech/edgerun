# TPM workflows

## TPM Key Initialization

The `edgerund init` command initializes TPM keys **natively** using the TSS2 ESAPI library (`tss-esapi` Rust crate) which wraps the system's `libtss2-esys.so`. This is the same code path that `tpm2-tools` uses internally.

When you run:
```bash
edgerund init --config node.yaml
```

The daemon:
1. Detects TPM availability via `/dev/tpmrm0`
2. Scans for available persistent handles (0x81000001-0x810000FF)
3. Creates an ECDSA P-256 signing key using TSS2 ESAPI:
   - `Esys_CreatePrimary` - Creates the primary ECC key under owner hierarchy with proper attributes
   - `Esys_EvictControl` - Persists the key to a permanent handle
4. Derives the NodeID from the public key coordinates
5. Writes the configuration with the TPM signer type and handle

**Requires:** `tpm2-tss` system library (provides `libtss2-esys.so`). On Arch/CachyOS: `pacman -S tpm2-tss`.

### NV Space

TPM persistent handles consume NV storage. If you get an "insufficient space for NV allocation" error, evict unused handles:
```bash
tpm2_getcap handles-persistent          # List existing handles
tpm2_evictcontrol -C o -c 0x81000001    # Evict a specific handle
```

## TPM Protocol Implementation Details

### Why TSS2 ESAPI?

The kernel TPM resource manager (`/dev/tpmrm0`) has several restrictions that make raw TPM command construction difficult:

1. **Session creation blocked**: `TPM2_StartAuthSession` is intercepted and rejected by the kernel RM
2. **Handle virtualization**: The RM maps actual TPM handles to virtual handles, requiring special handling for `EvictControl`
3. **Response format differences**: The RM modifies response structures (no `parameterSize` field in SESSIONS responses)

The TSS2 ESAPI library handles all these quirks correctly, making it the reliable choice for TPM key provisioning.

### Architecture

```
edgerund init
  └─> TpmDevice::create_ecdsa_p256_signing_key()
       └─> tss-esapi (Rust crate)
            └─> libtss2-esys.so (system library)
                 └─> /dev/tpmrm0 (TPM resource manager)
```

### Runtime Signing

After initialization, runtime signing operations use the **native** TPM command implementation in `edgerun-tpm` which communicates directly with `/dev/tpmrm0`. This works because:
- The persistent key handle is already established
- `TPM2_ReadPublic` and `TPM2_Sign` don't require session creation
- The RM allows these commands with password auth sessions

### Key Attributes

Created keys have these TPM object attributes:
- `signEncrypt` - Key can be used for signing
- `fixedTPM` - Key cannot be duplicated to another TPM
- `fixedParent` - Key cannot be reparented
- `sensitiveDataOrigin` - Key material generated inside TPM
- `userWithAuth` - Key can be authorized with empty password
- `noDA` - Key immune to dictionary attacks
