# Node Boot Initramfs Integration V1

## Goal
- Bring `/home/ken/src/initramfs-min` into this repository as the canonical node-boot initramfs source.
- Provide a deterministic in-repo build entrypoint to package initramfs artifacts into `out/`.
- Make kernel/initramfs tuning changes reviewable in this repository instead of an external tree.

## Non-goals
- No full kernel compilation pipeline in this change.
- No UKI signing/enrollment automation in this change.
- No production boot policy change beyond packaging/build workflow.

## Security and constraints
- Build outputs must be written under `out/` only.
- Source of truth must be in-repo (`os/initramfs-min`), not mutable external paths.
- Keep runtime behavior explicit in `init` and `etc/init.d/rcS` with no hidden bypass flags.

## Acceptance criteria
- `os/initramfs-min/` exists and contains the imported initramfs source tree (excluding VCS metadata).
- A script builds `out/nodeos/initramfs.cpio.gz` from `os/initramfs-min`.
- Makefile exposes a target for initramfs packaging.
- Build validation and lint/shell validation for the new script pass.

## Rollout and rollback
- Rollout: vendor source tree, add build script and Makefile target, validate artifact generation.
- Rollback: remove `os/initramfs-min`, script, and Makefile target.
