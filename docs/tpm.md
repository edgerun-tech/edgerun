# TPM workflows

## Inspect TPM capabilities

List the TPM algorithms, curves, and persistent handles through the CLI:

```bash
(cd go && go run ./cmd/lifegraphctl --mode tpm-capabilities)
```

Optional explicit TPM device:

```bash
(cd go && go run ./cmd/lifegraphctl --mode tpm-capabilities --tpm-path /dev/tpmrm0)
```

Inspect a persistent handle and get the recommended wire signature algorithm:

```bash
(cd go && go run ./cmd/lifegraphctl --mode tpm-key-info --target 0x81010020)
```

## Provision a direct-signing TPM key

Create and persist an ECDSA P-256 key:

```bash
./scripts/tpm_provision_key.sh \
  --type ecc-p256 \
  --handle 0x81010020 \
  --out-dir ./var/tpm-node-server
```

Create and persist an RSA-2048 key:

```bash
./scripts/tpm_provision_key.sh \
  --type rsa-2048 \
  --handle 0x81010021 \
  --out-dir ./var/tpm-node-server-rsa
```

The script writes:
- `public.pem`
- `metadata.env`
- `lifegraphd.env`
- `run-lifegraphd.sh`
- transient context artifacts for inspection/reprovisioning

`metadata.env` contains raw key settings. `lifegraphd.env` is shell-ready, and `run-lifegraphd.sh` launches the daemon with the matching flags in one command.

## Provision an auth-protected TPM key

Add `--auth` when creating the key:

```bash
./scripts/tpm_provision_key.sh \
  --type ecc-p256 \
  --handle 0x81010022 \
  --auth 'file:./secrets/node-server.auth' \
  --out-dir ./var/tpm-node-server-auth
```

Then run the daemon with the same authorization material:

```bash
source ./var/tpm-node-server-auth/metadata.env

(cd go && go run ./cmd/lifegraphd \
  --fixture node_server \
  --data-dir ../var/lifegraph \
  --tpm-key-context "$LIFEGRAPHD_TPM_KEY_CONTEXT" \
  --tpm-public-key "$LIFEGRAPHD_TPM_PUBLIC_KEY" \
  --signature-algorithm "$LIFEGRAPHD_SIGNATURE_ALGORITHM" \
  --tpm-key-auth "$LIFEGRAPHD_TPM_KEY_AUTH")
```

## Provision a PCR-policy TPM key

Create a reusable PCR policy digest:

```bash
./scripts/tpm_make_pcr_policy.sh \
  --pcr-list sha256:7 \
  --out-dir ./var/tpm-policy
```

Create a key bound to that policy:

```bash
./scripts/tpm_provision_key.sh \
  --type ecc-p256 \
  --handle 0x81010023 \
  --policy ./var/tpm-policy/policy.digest \
  --out-dir ./var/tpm-node-server-policy
```

Before starting the daemon, open a live policy session that satisfies the same PCR rule:

```bash
./scripts/tpm_start_pcr_policy_session.sh \
  --pcr-list sha256:7 \
  --session ./var/tpm-policy/signing.session
```

Then point `lifegraphd` at that session using the normal auth flag:

```bash
source ./var/tpm-node-server-policy/metadata.env

(cd go && go run ./cmd/lifegraphd \
  --fixture node_server \
  --data-dir ../var/lifegraph \
  --tpm-key-context "$LIFEGRAPHD_TPM_KEY_CONTEXT" \
  --tpm-public-key "$LIFEGRAPHD_TPM_PUBLIC_KEY" \
  --signature-algorithm "$LIFEGRAPHD_SIGNATURE_ALGORITHM" \
  --tpm-key-auth session:../var/tpm-policy/signing.session)
```

When finished, flush the live session:

```bash
tpm2_flushcontext ./var/tpm-policy/signing.session
```

## Run lifegraphd with direct TPM signing

ECDSA P-256 example:

```bash
source ./var/tpm-node-server/metadata.env

(cd go && go run ./cmd/lifegraphd \
  --fixture node_server \
  --data-dir ../var/lifegraph \
  --tpm-key-context "$LIFEGRAPHD_TPM_KEY_CONTEXT" \
  --tpm-public-key "$LIFEGRAPHD_TPM_PUBLIC_KEY" \
  --signature-algorithm "$LIFEGRAPHD_SIGNATURE_ALGORITHM")
```

The legacy TPM seed-loading mode has been removed. Use direct TPM signing with `--tpm-key-context`, `--tpm-public-key`, and `--signature-algorithm` instead.

## Supported selectable signing algorithms

- `ed25519`
- `ecdsa-p256`
- `ecdsa-p384`
- `rsa-pkcs1v15-sha256`
- `rsa-pss-sha256`

## Notes

- Direct TPM signing uses `tpm2_sign` with a TPM key context or persistent handle and exported PEM public key.
- `--tpm-key-auth` accepts normal TPM auth strings, including `file:` and `session:` formats supported by `tpm2-tools`.
- Verification happens in the Lifegraph runtime using the selected wire signature algorithm.
- ECDSA and RSA are now protocol-valid options, not just local experiments.

## Daemon startup TPM self-check

When direct TPM signing is enabled, `lifegraphd` now refuses to start unless all of these pass before it serves traffic:

- the TPM handle/context can be read
- the TPM public key matches the provided PEM file
- the selected `--signature-algorithm` matches the TPM key type
- a real sign+verify self-check succeeds with the TPM key
