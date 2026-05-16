# Capability Wire Records

Capability invocation artifacts are rkyv `SdkWireRecord` values:

- `SdkWireRecord::CapabilityRequest`
- `SdkWireRecord::CapabilityResponse`

The removed ECAP/ECRP byte layouts are not supported compatibility paths. CLI
commands that still use capability-oriented names write and read rkyv records.

## Request

`CapabilityRequest` carries:

- capability kind, operation, assurance, app id, release id
- subject hash and payload hash
- context bytes, payload bytes, nonce bytes

`payload_sha256` may pin bytes that are not carried inline when the provider can
obtain them through another capability or transport.

## Response

`CapabilityResponse` carries:

- request hash, capability kind, operation, status, assurance
- provider bytes, responder bytes, payload bytes, proof bytes

The generic verifier checks:

- `request_sha256 == sha256(request.rkyv)`
- response kind and operation match the request
- response assurance is at least the requested assurance

Capability-specific verifiers then interpret `payload` and `proof`.

## Status

```text
0 ok
1 policy_denied
2 invalid_request
3 provider_failed
```

For nonzero status, `payload` is a reason byte string and `proof` is the sha256
of an archived `CapabilityResponseProofRecord`.

## Initial Kinds

```text
1 signing
2 sealing
3 payment
4 storage
5 network
```

## Signing

Signing uses:

```text
capability_kind = 1
operation = 1
request.context = signing domain bytes
request.payload = archived SigningAlgorithmRecord
response.responder = signer public key bytes
response.payload = archived SigningAlgorithmRecord
response.proof = signature bytes
```

For the current Ed25519 CLI signer, `proof` signs:

```text
"edgerun-sdk.rkyv.v1.capability-response" || sha256(request.rkyv)
```

Browser passkeys should use the same request hash in the WebAuthn challenge
input, with provider and assertion bytes carried opaquely in the response.

## Sealing

Sealing uses:

```text
capability_kind = 2
operation = 3 seal
operation = 4 unseal
request.context = policy/context bytes
request.payload = plaintext bytes for seal, sealed envelope bytes for unseal
response.responder = runtime sealing key identifier bytes
response.payload = sealed envelope bytes for seal, plaintext bytes for unseal
response.proof = sha256 capability proof bytes
```

The SDK CLI runtime provider uses `edgerun-seal`, which seals with AES-GCM under
a 256-bit runtime key. The key is supplied to the runtime command and is not
part of the deterministic wasm/app request.

The sealing proof is the sha256 of an archived `CapabilityResponseProofRecord`.

With the sealing key, `verify-seal-response` can additionally check:

- seal response payload decrypts to request payload
- unseal response payload equals decrypt(request payload)

## Storage

Storage uses:

```text
capability_kind = 4
operation = 6 read
operation = 7 write
request.context = storage key / namespace bytes
request.payload = bytes to write, or empty for read
request.payload_sha256 = write payload hash, read expected hash, or zero for any read
response.payload = read bytes for read, or archived StorageWriteReceiptRecord for write
response.proof = sha256 capability proof bytes
```

The SDK CLI local provider maps `sha256(request.context)` to an object filename
inside a supplied root directory. Browser localStorage, Google Drive, and remote
stores can use the same rkyv request/response shapes with different provider
implementations.
