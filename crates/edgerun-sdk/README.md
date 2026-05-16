# edgerun-sdk

`edgerun-sdk` is the SDK surface for building Edgerun apps from deterministic
WebAssembly units.

The SDK contract is not Rust. Rust can produce units, but the app boundary is:

```text
unit.wasm + manifest.edm + exact dependency hashes
```

Each unit implements one standard, algorithm, protocol fragment, or app
capability. Units expose their own typed API and compose through wasm
exports. The shared ABI only defines identity, determinism, byte-oriented
invocation, and manifest verification. Units own their memory; composers copy
values between unit APIs instead of sharing memory.

See `docs/standard-module-v1.md` for the first ABI model.
See `docs/edum-v1.md` for the unit manifest record.
See `docs/eapi-v1.md` for the function API manifest record.
See `docs/ecmp-v1.md` for the composition manifest record.
See `docs/edrr-v1.md` for the execution report record.
See `docs/eseg-v1.md` for the distributed segment manifest record.
See `docs/esrr-v1.md` for the distributed segment report record.
See `docs/esig-v1.md` for the segment report signature sidecar record.
See `docs/espk-v1.md` for the segment signer policy record.
See `docs/echn-v1.md` for the distributed chain manifest record.
See `docs/capability-units-v1.md` for the capability invocation units mapped
from the existing Edgerun capability crates.

The `.edm`, `.edr`, `.eseg`, `.esrr`, `.esig`, `.espk`, and `.echn` files in
this crate are rkyv archives of concrete `SdkWireRecord` variants. Do not add
magic-header encoders or compatibility readers for the removed SDK byte formats.

## Current Units

- `sha256-fips180`: FIPS 180 SHA-256 digest unit.
- `sha384-fips180`: FIPS 180 SHA-384 digest unit.
- `sha512-fips180`: FIPS 180 SHA-512 digest unit.
- `rfc2104-pad`: RFC 2104 HMAC key pad unit.
- `hmac-sha256-rfc2104`: RFC 2104 HMAC-SHA256 unit.
- `capability-policy-v1`: scalar authorization checks from
  `edgerun-capabilities::policy`.
- `capability-session-v1`: remote capability session checks from
  `edgerun-remote-capability`.
- `capability-provider-v1`: provider invocation/result frame checks from
  `edgerun-remote-capability`.
- `ipv4-rfc791`: IPv4 packet parse unit.
- `udp-rfc768`: UDP datagram parse unit.
- `tftp-rfc1350`: TFTP message parse/validate unit.
- `dns-rfc1035`: DNS header parse unit.
- `http-token-rfc9110`: HTTP token validation unit.
- `utf8-rfc3629`: UTF-8 validation unit.
- `base64url-rfc4648`: Base64url encode/decode unit without padding.
- `cbor-rfc8949`: CBOR item header parse unit.
- `byte-tools-v1`: Byte equality, prefix, find, and ASCII lowercase helpers.
- `http-field-rfc9110`: HTTP field-line parse unit.
- `decision-byte-v1`: one-byte private policy decision helper.
- `constant-time-eq-v1`: constant-time byte equality status helper.

## Current Compositions

- `hmac-sha256-rfc2104-composed`: hash-pinned composition over
  `rfc2104-pad` and `sha256-fips180`.
- `hkdf-extract-sha256-rfc5869`: RFC 5869 HKDF-Extract composition over
  `hmac-sha256-rfc2104`.
- `hkdf-expand-sha256-rfc5869-l42`: RFC 5869 HKDF-Expand composition over
  `hmac-sha256-rfc2104` for the two-block `L=42` profile.
- `http-auth-preflight-rfc9110`: HTTP authorization header-name preflight over
  `http-field-rfc9110` and `byte-tools-v1`.
- `auth-decision-private-v1`: private decision composition over
  `decision-byte-v1`.
- `hmac-sha256-verify-rfc2104`: RFC 2104 HMAC-SHA256 tag verification over
  `hmac-sha256-rfc2104` and `constant-time-eq-v1`.

## Current Segments

- `http-auth-preflight-public-edge-v1`: public-edge segment over
  `http-auth-preflight-rfc9110`, binding two inputs and one output to the
  full HTTP preflight composition range.
- `auth-decision-private-node-v1`: private-policy-node segment over
  `auth-decision-private-v1`, binding a linked status byte and a private
  expected byte to one output status byte.
- `hmac-sha256-verify-private-node-v1`: private-policy-node segment over
  `hmac-sha256-verify-rfc2104`, binding a private key, linked message, and
  linked expected tag to one verification status byte.

## Current Chains

- `http-auth-preflight-chain-v1`: one-segment chain over
  `http-auth-preflight-public-edge-v1`.
- `http-auth-decision-chain-v1`: two-segment chain linking the public HTTP
  preflight output into the private decision segment.

## Commands

```bash
edgerun-sdk list
edgerun-sdk build-artifacts
edgerun-sdk verify
edgerun-sdk verify-segment
edgerun-sdk verify-chain
edgerun-sdk explain tftp-rfc1350
edgerun-sdk generate-unit-metadata http-field-rfc9110
edgerun-sdk run-segment http-auth-preflight-public-edge-v1 \
  417574686f72697a6174696f6e3a2042656172657220616263 \
  617574686f72697a6174696f6e
edgerun-sdk run-segment auth-decision-private-node-v1 00 00
edgerun-sdk run-segment hmac-sha256-verify-private-node-v1 \
  4a656665 \
  7768617420646f2079612077616e7420666f72206e6f7468696e673f \
  5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
edgerun-sdk run-composition hmac-sha256-rfc2104-composed \
  4a656665 \
  7768617420646f2079612077616e7420666f72206e6f7468696e673f
edgerun-sdk run-composition hmac-sha256-verify-rfc2104 \
  4a656665 \
  7768617420646f2079612077616e7420666f72206e6f7468696e673f \
  5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
edgerun-sdk quote-composition hmac-sha256-rfc2104-composed 4 28
edgerun-sdk preflight-composition http-auth-preflight-rfc9110 25 13
edgerun-sdk verify-report compositions/hmac-sha256-rfc2104/report.edr
edgerun-sdk verify-segment-report segments/http-auth-preflight-public-edge-v1/report.esrr
edgerun-sdk replay-segment-report segments/hmac-sha256-verify-private-node-v1/report.esrr \
  4a656665 \
  7768617420646f2079612077616e7420666f72206e6f7468696e673f \
  5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
edgerun-sdk sign-segment-report \
  segments/hmac-sha256-verify-private-node-v1/report.esrr \
  000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
edgerun-sdk write-signer-policy policy.espk \
  hmac-sha256-verify-private-node-v1 \
  03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8
edgerun-sdk verify-signed-segment-report \
  segments/hmac-sha256-verify-private-node-v1/report.esrr \
  segments/hmac-sha256-verify-private-node-v1/report.esig \
  policy.espk
edgerun-sdk verify-chain-reports http-auth-preflight-chain-v1 \
  segments/http-auth-preflight-public-edge-v1/report.esrr
edgerun-sdk verify-chain-reports http-auth-decision-chain-v1 \
  segments/http-auth-preflight-public-edge-v1/report.esrr \
  segments/auth-decision-private-node-v1/report.esrr
```

`verify` checks unit wasm hashes, rkyv unit manifests, rkyv API manifests, exact wasm
import/export signatures, no shared-memory imports, and rkyv composition
manifests.

`build-artifacts` discovers Rust unit crates at `units/<id>/rust/Cargo.toml`,
builds them for `wasm32-unknown-unknown`, derives rkyv unit/API records from the resulting
wasm exports, then reads each composition's `compose.edsl`, each segment's
`segment.edsl`, and each chain's `chain.edsl` to write deterministic
rkyv composition/segment/chain artifacts. WAT units and generated Rust registries are not part of
the unit creation path.

`generate-unit-metadata` builds the Rust source for one unit, then derives
deterministic `manifest.edm` and `api.edm` from the compiled wasm.

`run-composition` accepts hex-encoded inputs and prints component hashes, input
lengths, deterministic cost, output bytes, output SHA-256, and writes
`report.edr`.

`quote-composition` accepts only input lengths and returns the deterministic
cost, output length, and executed step count for the branch path implied by
those lengths.

`preflight-composition` accepts only input lengths and validates composition
shape without running wasm computation. It returns cost bounds, output length
when knowable, and byte-dependent scalar or branch points.

`run-segment` executes a discovered distributed segment locally and writes an
`ESRR` segment report. `verify-segment-report` checks the report against the
segment hash and length-only preflight bounds. `replay-segment-report` takes
the original inputs, re-executes the pinned composition, and verifies the
reported output hash.

`sign-segment-report` writes an `ESIG` sidecar that signs the exact ESRR bytes.
`verify-signed-segment-report` validates the ESRR preflight bindings and the
Ed25519 signature over the report hash. If a policy path is supplied, it also
requires an `ESPK` signer policy entry that binds the signature public key to
the segment id, segment hash, node role, and capability.

`verify-report` checks a binary `report.edr` against the discovered composition
hash, component hashes, public input lengths, deterministic quote cost, output
length, and output SHA-256 binding.
