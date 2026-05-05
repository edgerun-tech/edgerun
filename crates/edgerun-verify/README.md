# edgerun-verify

`edgerun-verify` is the EdgeRun v0 protocol verification boundary.

It answers one narrow question:

> Does this protocol record have a valid EdgeRun v0 signature over its canonical signable bytes?

It does **not** decide whether the record should be trusted, accepted, executed, cached, routed through, or promoted into local state. Those decisions belong to local policy, stream validation, delegation validation, route policy, and application logic.

## Verified families

The crate currently verifies signatures for these protocol families:

- `EventEnvelope`
- `CommandEnvelope`
- `DelegationRecord`
- `RevocationRecord`
- `IdentityRecord`
- `AssuranceClaim`
- `RouteAdvertisement`

`RouteAdvertisement` verification is advisory only. A valid route signature proves attribution and tamper evidence, not route authority or route selection.

## Not verifier scope

Transport/session records are intentionally not part of the core protocol verifier:

- `SessionHello`
- `SessionAccept`
- `RelayEnvelope`

Those are transport mechanics. They may have transport-specific authentication, but they are not durable protocol authority records.

## Future candidates

These may become verifier families once their custom-wire canonical encoders are implemented:

- `SnapshotDescriptor`
- `QueryRequest`
- `QueryResultFragment`

Until then, they must not be accepted through placeholder canonicalization.

## Cryptographic model

For v0, the protocol verifier uses the canonical crypto helpers from `edgerun-core::crypto`:

```text
record
  -> canonical signable bytes
  -> SHA-256(hash_domain || 0x00 || canonical_bytes)
  -> record_hash
  -> verify ECDSA P-256 over sig_domain || 0x00 || record_hash
```

The accepted signature algorithm is currently:

```text
SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 = 1
```

Public keys may be supplied as either:

- raw P-256 `x || y` bytes, 64 bytes
- SEC1 P-256 public key bytes

## Main API

```rust
use edgerun_verify::{ProtocolSignerRef, verify_event_envelope};

let verification = verify_event_envelope(
    &event,
    ProtocolSignerRef::P256Raw64(&writer_public_key),
)?;
```

Family-specific helpers:

```rust
verify_event_envelope(...)
verify_command_envelope(...)
verify_delegation_record(...)
verify_revocation_record(...)
verify_identity_record(...)
verify_assurance_claim(...)
verify_route_advertisement(...)
```

Generic helpers:

```rust
verify_signed_record(...)
verify_protocol_record(...)
verify_protocol_record_hw(...)
protocol_record_hash(...)
protocol_signature_input(...)
protocol_signable_bytes(...)
```

`verify_protocol_record_hw` exists for signatures produced by hardware signers that sign a 32-byte prehash of the protocol signature input.

## Important boundary

Verification is not authorization.

A successful verification means:

- the canonical bytes were reconstructed for the supported protocol family
- the record hash was derived with the correct family hash domain
- the signature was checked with the correct family signature domain
- the supplied public key verifies the signature

A successful verification does **not** mean:

- the issuer is trusted
- a delegation chain is valid
- a command should be committed
- a route should be used
- an assurance claim satisfies local policy
- a stream is valid as a whole

This crate should be used by higher-level validators as the cryptographic verification primitive, not as a policy engine.
