# Codec Primitive Corpus

This directory contains small JSON fixtures for the WAT codec primitive modules
under `standards/build/wasm/codec-primitives/`.

The older protocol corpora in this repository use TOML case indexes with `.hex`
payload files. Codec primitives also need scalar expectations, output lengths,
function names, and mixed ASCII/hex inputs, so this directory uses the explicit
`edgerun.codec-primitives.corpus.v1` JSON schema marker instead.

Each fixture file is intentionally small and auditable. The smoke runners do
not consume these files yet; they are corpus seeds for future parity and
conformance runners.
