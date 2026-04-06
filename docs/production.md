# Production deployment notes

This repository can now support a **controlled production-style deployment** of the Rust daemon, but it is still best treated as a reference implementation with a hardened baseline rather than a fully finished internet-scale platform.

## Included hardening baseline

`lifegraphd` (from `lifegraph-node` crate) supports:
- disk-backed object storage with runtime state persistence
- graceful shutdown on `SIGINT` / `SIGTERM`
- object encryption via AES-GCM with persistent blob keys
- local TPM-backed identity/signing via direct TPM signing
- selective replication and receiver-driven transport

## Recommended deployment profile

Minimum recommendation for a real deployment:
1. Run `lifegraphd` with `--data-dir` on durable storage.
2. Restrict network exposure to trusted peers or an internal network segment.
3. Ship stderr logs into your observability stack.
4. Back up the data directory, including the SQLite index database and encrypted blob store.
5. Pin and monitor body-size and timeout limits appropriate to your workload.

## Automation and packaging

Operational support in-repo now includes:
- `Makefile` targets for `check`, `test`, `build`, `release-build`, and `docker-build`
- `.github/workflows/ci.yml` to continuously run Rust tests and builds
- `.github/workflows/release.yml` to build tagged multi-arch Linux daemon artifacts for release handoff
- a multi-stage `Dockerfile` for building a minimal `lifegraphd` container image

Container build:

```bash
docker build -t lifegraph-reference-core:local .
make release-build
```

Container run example for explicit local testing:

```bash
docker run --rm -p 8080:8080 \
  -v "$PWD/var/lifegraph:/var/lib/lifegraph" \
  lifegraph-reference-core:local
```

## Example systemd-style command

```bash
exec ./lifegraphd \
  --data-dir /var/lib/lifegraph \
  --config /etc/lifegraph/node.yaml
```

## Still not "done forever"

You should still plan for additional work before calling this a fully mature external-facing service:
- richer metrics and tracing
- formal secret rotation and multi-tenant auth models
- stronger external KMS/HSM-backed key management
- larger conformance, fuzz, and interoperability test programs
- performance characterization under expected production load
- security review of protocol and deployment profile

## Backup / restore / upgrade playbooks

### Backup

1. Stop writes if you need a crash-consistent snapshot.
2. Copy the entire data directory, including:
   - encrypted blob directories
   - SQLite index database
   - event log files
3. Record the daemon version/commit and the protobuf/schema version in use.
4. Store any hardware-sealed blob keys in your secret manager.

### Restore

1. Restore the data directory to the target host.
2. Restore any hardware-sealed blob keys if applicable.
3. Start the daemon and validate expected state through local query/inspection.

### Upgrade

1. Run:
   - `make test`
   - `./scripts/check_interop.sh`
2. Take a fresh backup.
3. Roll the new binary with the same data directory and secrets.
4. Re-run smoke checks and verify metrics/logs stay healthy.
