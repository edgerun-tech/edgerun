# TPM workflows

Status: implemented in code for raw TPM command provisioning/signing through
`/dev/tpmrm0`; host-Linux only. TSS2 ESAPI is not used by the current
`edgerun-tpm` crate.

## TPM Key Initialization

The `edgerund init` command initializes TPM keys using raw TPM wire commands
sent directly to `/dev/tpmrm0`.

When you run:
```bash
edgerund init --config node.yaml
```

The daemon:
1. Detects TPM availability via `/dev/tpmrm0`
2. Scans for available persistent handles (0x81000001-0x810000FF)
3. Creates an ECDSA P-256 signing key using raw TPM commands:
   - `CreatePrimary` - Creates the primary ECC key under owner hierarchy
   - `EvictControl` - Persists the key to a permanent handle
4. Derives the NodeID from the public key coordinates
5. Writes the configuration with the TPM signer type and handle

**Requires:** Linux TPM resource manager device access through `/dev/tpmrm0`.

### NV Space

TPM persistent handles consume NV storage. If you get an "insufficient space for NV allocation" error, evict unused handles:
```bash
tpm2_getcap handles-persistent          # List existing handles
tpm2_evictcontrol -C o -c 0x81000001    # Evict a specific handle
```

## TPM Protocol Implementation Details

### Why Raw TPM Commands?

The current implementation avoids a runtime dependency on TSS2 ESAPI and uses
workspace-owned TPM wire builders/parsers. The kernel TPM resource manager
(`/dev/tpmrm0`) has restrictions that the code handles directly:

1. **Session creation blocked**: `TPM2_StartAuthSession` is intercepted and rejected by the kernel RM
2. **Handle virtualization**: The RM maps actual TPM handles to virtual handles, requiring special handling for `EvictControl`
3. **Response format differences**: The RM modifies response structures (no `parameterSize` field in SESSIONS responses)

### Architecture

```
edgerund init
  └─> TpmDevice::create_ecdsa_p256_signing_key()
       └─> raw TPM command builders/parsers
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
