# edgerun v0.1 Initial Vector Corpus

This corpus is the first conformance seed set.

It contains 48 mandatory cases:

- canonical: 7
- stream: 7
- delegation: 7
- command: 9
- control: 6
- snapshot: 7
- object: 5

Each case contains:

- `manifest.yaml`
- `semantic_input.yaml`
- `local_state.yaml`
- `expected.yaml`

The canonical suite includes committed `canonical_signable.hex`, `canonical_full.hex`, `record_hash.hex`, `signature_input.hex`, and `signature.hex` artifacts for the signable-vs-full record. The delegation suite carries real signed links whose record hashes are derived by the validator from link contents. The command, stream, control, and snapshot suites now also carry signed semantic records or descriptors validated from derived hashes. The object suite now enforces claimed object-id and representation-digest consistency when those claims are present.

## Suites

- `canonical/`
- `stream/`
- `delegation/`
- `command/`
- `control/`
- `snapshot/`
- `object/`

## Fixture identities

Suggested fixture ids used in semantic inputs:

- `user_alice`
- `user_bob`
- `node_phone`
- `node_server`
- `agent_assistant`

These fixture ids are symbolic in the first draft corpus.
