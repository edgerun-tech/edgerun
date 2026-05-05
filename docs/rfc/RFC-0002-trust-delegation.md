# RFC-0002: Trust & Delegation

**Status:** Working Draft
**Date:** 2026-04-12
**Based on:** Protocol spec §7, §14.5-14.10 + `edgerun-capabilities`, `edgerun-capability-policy`, `edgerun-remote-capability`

---

## Abstract

The trust and delegation system implements capability-based authority, signed delegation chains, revocation processing, and assurance normalization. It maps to protocol spec sections 7 (Trust & Delegation) and 14.5-14.10 (schema definitions).

---

## Components

### edgerun-capabilities

**Status:** ⚠️ Partial (structural validation only, no session lifecycle)
**Tests:** ~50+ passing (874 lines of test code, 130 lines of impl)

See [RFC-0001](RFC-0001-protocol-core.md#edgerun-capabilities--capability-model-abstraction) for full analysis.

**Key Finding:** 87% test code vs 13% implementation code. Thoroughly tested structural validation but no session lifecycle, no policy engine, no revocation processing.

### edgerun-capability-policy

**Status:** ✅ Functional (100 tests, 1891 lines of tests, 806 lines of impl across 6 files)

**Purpose:** Grant evaluation, constraint enforcement, access checking. The actual policy engine that `edgerun-capabilities` delegates to.

### edgerun-remote-capability

**Status:** ✅ Functional
**Tests:** Part of workspace

**Purpose:** Remote capability negotiation. Implements `RemoteCapabilityProvider` trait with session open/invoke/close lifecycle — the methods missing from `edgerun-capabilities`' `CapabilityProvider`.

## Spec Mapping

| Spec Section | Implementation | Status |
|---|---|---|
| §3.14 Capability | `CapabilityDescriptor`, `CapabilityGrant`, `CapabilityInvocation` | ✅ Proto types exist |
| §7 Trust & Delegation | `validate_grant`, `validate_descriptor` | ⚠️ Structural only |
| §7.1 Assurance Normalization | `AssuranceRequirement`, `AssuranceClaim` | ✅ Proto types exist |
| §7.2 Revocation Normalization | `RevocationRecord`, `RevocationRef` | ✅ Proto types exist |
| §7.3 Installed vs Ephemeral Authority | — | ❌ Not implemented |
| §14.5 CapabilityDescriptor | Builder + validator | ✅ |
| §14.6 ScopeDescriptor | Proto type | ✅ |
| §14.7 ConstraintSet | Proto type + constraint builders | ⚠️ Missing duration/rate_limit builders |
| §14.9 DelegationRecord | Proto type | ✅ |
| §14.10 RevocationRecord | Proto type | ✅ |
| §18.5 Delegation Chain Validation | — | ❌ Not implemented |

---

## Outstanding Work

1. **Delegation chain validation** (§18.5) — Signature verification, continuity, attenuation, revocation checks
2. **Session lifecycle** — `open_session`, `invoke`, `close_session` on `CapabilityProvider`
3. **Revocation processing** — Matching revocations against active delegations
4. **Installed vs ephemeral authority** — Distinguish durable control from one-time command authorization
5. **Assurance evaluation** — Hardware-backed keys, attested runtime verification
6. **Constraint builders** — Missing `duration_value` and `rate_limit` field builders
