# EdgeRun Tor WAT Port

This directory ports the pre-erobj Tor implementation from commit
`1b243c8d6a843585bc54b4366f814cf5f56615b1` into WebAssembly text.

`tor_library.wat` is the integration module. It exports one linear memory and
keeps the same public symbol names where practical, using pointer/length
parameters in the same order as the original assembly routines.

The source corpus copied from the baseline commit is under `source/`, with the
tracked file list in `source-files.txt`.

Current executable coverage:

- Tor constants and role/capability state.
- Guard material validation and storage.
- Fixed and variable cell field load/store helpers.
- EXTEND2 body builder.
- Relay BEGIN cell builder for IPv4:port streams.
- HSDir publish/fetch HTTP request builders.
- Base64/base32 helpers used by hidden-service descriptors.

Crypto and the complete live network state machines are being ported as
separate slices before final integration.
