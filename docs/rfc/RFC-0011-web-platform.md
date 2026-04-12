# RFC-0011: Web Platform Crates

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

Web platform crates provide generated type definitions for ECMAScript, Fetch, Encoding, URL, DOM Events, IndexedDB, Service Workers, Trusted Types, HTTP, TLS, XML, Web IDL, and UI Events. Most are generated from proto data; a few have hand-written implementations.

---

## Generated Type Crates (All from Proto)

| Crate | Proto Source | Status | Description |
|-------|-------------|--------|-------------|
| `edgerun-ecmascript` | ecmascript_objects, ecmascript_abstract_ops, ecmascript_globals | ✅ Generated | 127 intrinsics, 121 abstract ops, 27 built-in objects |
| `edgerun-fetch` | fetch types | ✅ Generated | Fetch API types |
| `edgerun-encoding` | 38 encodings + BOM | ✅ Generated | Encoding API types |
| `edgerun-url` | URL spec states | ✅ Generated | URL API types |
| `edgerun-indexeddb` | IndexedDB types | ✅ Generated | IndexedDB API types |
| `edgerun-service-workers` | Service worker types | ✅ Generated | Service Worker API types |
| `edgerun-trusted-types` | Trusted types | ✅ Generated | Trusted Types API |
| `edgerun-webidl` | Web IDL types | ⚠️ Partial | Web IDL type system |
| `edgerun-uievents` | UI Events key codes | ✅ Generated | Keyboard/mouse event codes |
| `edgerun-xml` | XML types | ⚠️ Partial | XML parsing types |

---

## Hand-Written Implementations

### edgerun-http

**Status:** ⚠️ Partial
**Tests:** H2 conformance analysis in progress

**Purpose:** HTTP implementation. Has detailed H2 spec analysis (`H2SPEC_ANALYSIS.md`) and conformance report (`CONFORMANCE_REPORT.md`).

### edgerun-tls

**Status:** ⚠️ Partial
**Tests:** RFC 8446 test vectors (`rfc8446_vectors.rs`)

**Purpose:** TLS 1.3 implementation. Includes RFC 8446 test vectors for cryptographic verification.

---

## edgerun-dom & edgerun-dom-events

**Status:** ✅ Generated

| Crate | Purpose |
|-------|---------|
| `edgerun-dom` | DOM interfaces as Rust traits (generated from WebIDL) |
| `edgerun-dom-events` | DOM event type definitions |

---

## edgerun-selectors

**Status:** ✅ Generated

CSS selector types: pseudo-classes, pseudo-elements, combinators, attribute selectors, specificity.

---

## Known Issues

1. **All generated, no implementations** — These crates provide types only, no behavioral code
2. **No JS engine** — `edgerun-ecmascript` defines 127 intrinsics and 121 abstract ops but no interpreter
3. **No network stack** — `edgerun-fetch` types exist but no actual HTTP client implementation
4. **No storage engine** — `edgerun-indexeddb` types exist but no IndexedDB implementation
5. **HTTP/TLS incomplete** — The two hand-written implementations are partial with conformance analysis in progress
6. **No DOM implementation** — `edgerun-dom` provides trait definitions but no actual DOM tree implementation
7. **No event loop** — No JavaScript event loop or task scheduling
