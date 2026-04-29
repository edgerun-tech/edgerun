# Security Policy

Edgerun Core is currently an alpha protocol/runtime implementation. Do not treat
the current release line as production-audited security software.

## Supported Versions

Security fixes are accepted on the default branch. Tagged alpha releases may be
superseded rather than backported unless a release branch is explicitly created.

## Reporting

Report suspected vulnerabilities privately through GitHub private vulnerability
reporting for `Sylchi/edgerun_reference_core` when available. If private
reporting is unavailable, contact the repository owner before filing a public
issue with exploit details.

Include:

- affected crate, binary, or protocol component;
- commit or release tag tested;
- reproduction steps or malformed input;
- expected and observed security boundary;
- whether private keys, signed streams, command commitment, delegation
  attenuation, object integrity, or query/access control are affected.

## Scope Notes

The protocol requires signed, single-writer streams, explicit command
commit/reject events, immutable object identity, attenuating delegation, and
identity-routed networking. Bugs that bypass those boundaries are security
issues even when they look like ordinary validation bugs.

Many hardware and bare-target paths are host-only, stubbed, or experimental.
Security reports should distinguish implemented code from protocol/design
requirements and generated reference material.
