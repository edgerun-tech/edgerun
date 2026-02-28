# Device Capability Bluetooth NFC Detection V1

## Goal
- Extend host capability probing to detect Bluetooth and NFC availability consistently across Linux, Android, and Windows.
- Keep core report shape stable while adding explicit capability signals and adapter-count metrics.

## Non-goals
- No active radio operations or connection attempts.
- No deep vendor stack interrogation (BlueZ D-Bus, Android HAL JNI, Windows UWP APIs) in this phase.

## Security and constraints
- Probes are read-only and side-effect free.
- Probe failures return explicit `Unknown`/`None`.
- Source attribution must remain deterministic and auditable.

## Acceptance criteria
- Core host capability model includes `bluetooth_inventory` and `nfc_inventory` signals.
- Core quantitative metrics include `bluetooth_adapter_count` and `nfc_adapter_count`.
- Linux/Android probing uses `/sys` and `/dev` surfaces for Bluetooth/NFC detection and counts.
- Windows probing uses service presence baseline (`bthserv`, `NfcSvc`) with explicit unknown handling.
- Detailed resolved-source output includes Bluetooth and NFC source paths.

## Rollout
1. Extend core schema fields.
2. Add host probes and metrics population.
3. Validate compile/tests and cross-target checks.

## Rollback
- Remove added fields and related host probes.
