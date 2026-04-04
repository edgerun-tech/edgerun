# Production deployment notes

This repository can now support a **controlled production-style deployment** of the Go daemon, but it is still best treated as a reference implementation with a hardened baseline rather than a fully finished internet-scale platform.

## Included hardening baseline

`go/cmd/lifegraphd` now supports:
- disk-backed object storage with runtime state persistence
- gRPC server over QUIC transport
- graceful shutdown on `SIGINT` / `SIGTERM`
- TLS via `--tls-cert` and `--tls-key`, with startup refusal for non-loopback plaintext listeners unless `--allow-insecure-transport` is explicitly set
- bounded gRPC/object message sizes
- stderr logging
- object encryption via `aes-gcm-v1` with `LIFEGRAPH_MASTER_KEY_HEX` and an explicit per-object access label
- insecure legacy `fixture-box-v0` support has been removed entirely
- local TPM-backed identity/signing via:
  - `--tpm-key-context` / `--tpm-public-key` / `--signature-algorithm` for direct TPM ECDSA/RSA signing

## Recommended deployment profile

Minimum recommendation for a real deployment:
1. Run `lifegraphd` with `--data-dir` on durable storage.
2. Enable TLS directly or place the daemon behind a TLS-terminating reverse proxy. Non-loopback plaintext listeners are refused by default.
3. Set `LIFEGRAPH_MASTER_KEY_HEX` to a 16/24/32-byte AES key encoded as hex and rotate it carefully.
4. If you use TPM-backed identity, make sure the TPM key context/public key pair is stable across restarts and backed by real TPM-managed storage.
5. Restrict network exposure to trusted peers or an internal network segment.
6. Ship stderr logs into your observability stack.
7. Back up the data directory, including `lifegraph.db`.
8. Pin and monitor body-size and timeout limits appropriate to your workload.

## Automation and packaging

Operational support in-repo now includes:
- `Makefile` targets for `test`, `interop`, `build-go`, and `docker-build`
- `.github/workflows/ci.yml` to continuously run Go tests, conformance checks, and a daemon build
- `.github/workflows/release.yml` to build tagged multi-arch Linux daemon artifacts for release handoff
- a multi-stage `Dockerfile` for building a minimal `lifegraphd` container image

Container build:

```bash
docker build -t lifegraph-reference-core:local .
make coverage-go
make release-build
```

Container run example for explicit local testing only:

```bash
docker run --rm -p 8080:8080 \
  -v "$PWD/var/lifegraph:/var/lib/lifegraph" \
  lifegraph-reference-core:local \
  --fixture node_server \
  --listen 0.0.0.0:8080 \
  --data-dir /var/lib/lifegraph \
  --allow-insecure-fixture-identity \
  --allow-insecure-transport
```

For non-local deployments, replace the insecure flags with TLS configuration.

## Example systemd-style command

```bash
export LIFEGRAPH_MASTER_KEY_HEX=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
exec ./lifegraphd \
  --fixture node_server \
  --listen 127.0.0.1:8443 \
  --data-dir /var/lib/lifegraph \
  --signature-algorithm ecdsa-p256 \
  --tpm-key-context /var/lib/lifegraph/node_server.ctx \
  --tpm-public-key /etc/lifegraph/node_server.pub.pem \
  --tls-cert /etc/lifegraph/tls.crt \
  --tls-key /etc/lifegraph/tls.key
```

For local-only testing with deterministic built-in fixture keys, add `--allow-insecure-fixture-identity`. Do not use that flag in production.

## Still not "done forever"

You should still plan for additional work before calling this a fully mature external-facing service:
- richer metrics and tracing
- formal secret rotation and multi-tenant auth models
- stronger external KMS/HSM-backed key management beyond env-backed AES keys
- the currently wired TPM-friendly signature algorithms are Ed25519, ECDSA P-256/P-384, RSA PKCS#1 v1.5 SHA-256, and RSA-PSS SHA-256
- larger conformance, fuzz, and interoperability test programs
- performance characterization under expected production load
- security review of protocol and deployment profile

## Backup / restore / upgrade playbooks

### Backup
1. Stop writes if you need a crash-consistent snapshot.
2. Copy the entire data directory, including:
   - object store directories
   - `lifegraph.db`
   Or use:
   - `go run ./cmd/lifegraphctl --mode sqlite-export --data-dir /var/lib/lifegraph --path /var/backups/lifegraph-export`
   - this writes `export-manifest.json` with SHA-256 checksums for the exported DB and blob files
3. Record the daemon version/commit and the protobuf/schema version in use.
4. Store the corresponding `LIFEGRAPH_MASTER_KEY_HEX` in your secret manager, not inside the backup archive.

### Restore
1. Restore the data directory to the target host.
   Or use:
   - `go run ./cmd/lifegraphctl --mode sqlite-import --data-dir /var/lib/lifegraph --path /var/backups/lifegraph-export`
   - import verifies `export-manifest.json` checksums before restoring
2. Restore `LIFEGRAPH_MASTER_KEY_HEX` before starting the daemon if encrypted objects were stored with `aes-gcm-v1`.
3. Start the daemon and validate expected gRPC object fetch / runtime inspection flows with `lifegraphctl`.

### Upgrade
1. Run:
   - `./scripts/check_interop.sh`
   - `(cd go && go test ./...)`
2. Take a fresh backup.
3. Roll the new binary with the same data directory and secrets.
4. Re-run smoke checks and verify metrics/logs stay healthy.

## Operator checks

Basic smoke checks:

```bash
lifegraphctl --base quics://127.0.0.1:8443 --mode sessions
lifegraphctl --base quics://127.0.0.1:8443 --mode routes
```

Encrypted object example:

```bash
# store with TLS and an explicit access label
OBJECT_HEADERS="$(mktemp)"
Use `lifegraphctl` against the QUIC transport for store/fetch flows.
```

The `access` label is an application-facing label you choose per encrypted object. It must match on write and read, and `LIFEGRAPH_MASTER_KEY_HEX` must also be present and correct on the daemon.
