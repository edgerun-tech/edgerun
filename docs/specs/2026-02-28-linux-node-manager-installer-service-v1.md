# Linux Node Manager Installer And Service V1

## Goal
- Provide a concrete Linux installer flow for `edgerun-node-manager` with deterministic service setup.
- Make onboarding command output map to an actual script that installs binary and systemd unit.

## Non-goals
- Packaging for non-systemd distributions.
- Deb/RPM packaging in this slice.
- Auto-bonding/auto-pairing logic inside installer.

## Security and Constraints
- Installer must not bind non-loopback by default; service uses `--local-bridge-listen 127.0.0.1:7777`.
- Installer must fail fast on download/install/write errors.
- Keep shell script deterministic and dependency-minimal (`curl`, `install`, `systemctl` expected).
- No port `8080` usage.

## Acceptance Criteria
1. `scripts/install-node-manager.sh` exists and is executable.
2. Script installs `edgerun-node-manager` binary to `/usr/local/bin/edgerun-node-manager` (default) from configurable URL.
3. Script installs/updates `edgerun-node-manager.service` under `/etc/systemd/system/` with loopback bridge bind.
4. Intent UI Linux onboarding command references the installer invocation path.
5. Cypress assertion verifies user-visible onboarding script includes installer invocation and service enable command.

## Rollout / Rollback
- Rollout:
1. Add script + systemd template.
2. Update onboarding command text.
3. Update Cypress expectations.
- Rollback:
1. Remove installer/template and restore prior onboarding command output.
