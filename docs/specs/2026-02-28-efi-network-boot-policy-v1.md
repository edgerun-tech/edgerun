# EFI Network Boot Policy V1

## Goal
- Add an EdgeRun-controlled EFI bootloader path where a single signed EFI application is deployed once and remains stable.
- At boot, device retrieves policy from `https://api.edgerun.tech` using TPM-backed device identity and signed request payloads.
- Server policy selects Linux boot target (`kernel` + `initramfs` + cmdline policy) so operators can change boot behavior from frontend/control plane without reflashing EFI binaries.

## Non-goals
- No support for non-Linux OS targets in V1.
- No unsigned/offline boot policy override path in production mode.
- No insecure runtime bypass for signature verification, certificate pinning, or rollback checks.

## Security and constraints
- Bootloader binary is signed once via Secure Boot trust chain and verified by firmware.
- Bootloader must pin EdgeRun API TLS trust (certificate or SPKI pin set) for `api.edgerun.tech`.
- Device request must include:
  - device public key,
  - nonce/timestamp,
  - monotonic boot counter,
  - TPM-backed signature over canonical request bytes.
- Policy response must be signed by EdgeRun control-plane signing key and verified in bootloader before use.
- Response must include anti-rollback field (`rollback_index`) and expiry (`valid_until_unix_s`).
- Bootloader only accepts Linux payload descriptors:
  - `kernel_url`,
  - `kernel_sha256`,
  - `initramfs_url`,
  - `initramfs_sha256`,
  - `cmdline`.
- `api_base` remains locked to `https://api.edgerun.tech`.
- Failure policy is `fail-closed`: on policy fetch/verification failure, do not continue boot.
- Port `8080` must not be used.

## Architecture
1. `edgerun-bootloader-efi` (new crate):
   - UEFI executable (`x86_64-unknown-uefi`) providing:
     - TPM identity init/load,
     - HTTPS policy fetch,
     - signed boot request submission,
     - signed policy verification,
     - kernel+initramfs fetch/verify,
     - Linux handoff.
2. Shared boot policy schema:
   - canonical serialization format for request/response signing.
3. Control-plane contract:
   - frontend chooses per-device or per-fleet boot target.
   - API returns signed policy object to device at boot.
4. Device persistent state:
   - store last accepted rollback index and last-known-good policy metadata in TPM NV.

## Acceptance criteria
- New spec-approved API contract for boot request/response exists and is versioned.
- New bootloader crate exists and builds for `x86_64-unknown-uefi`.
- Bootloader can:
  - derive/load TPM-backed identity,
  - submit signed boot request to `api.edgerun.tech`,
  - verify signed policy response,
  - download kernel/initramfs,
  - verify SHA-256 digests,
  - boot Linux with approved cmdline.
- Bootloader rejects:
  - expired policy,
  - invalid control-plane signature,
  - rollback index decrease,
  - hash mismatch for kernel/initramfs.
- Existing node boot scripts remain compatible with signed artifact flow under `out/`.

## Rollout
1. Phase A: protocol + control-plane endpoint contract + request/response signing in existing node-manager path for integration proving.
2. Phase B: new EFI crate with signed policy verification and Linux handoff.
3. Phase C: frontend boot-target controls wired to policy issuance.
4. Phase D: migration to EFI-first path on selected hardware, then broad rollout.

## Rollback
- Revert to currently signed UKI + `edgerun-node-manager` PID1 flow.
- Disable EFI policy endpoint issuance for affected device cohorts.
- Preserve rollback index state to prevent downgrade attacks during recovery.

## Spec alignment notes
- Aligns with existing repository secure boot and TPM identity direction:
  - `docs/specs/NODE_SECURE_BOOT_SIGNED_UKI_V1.md`
  - `crates/edgerun-node-manager`
- V1 keeps Linux-only boot targets and separates kernel/initramfs artifacts for operational simplicity.
