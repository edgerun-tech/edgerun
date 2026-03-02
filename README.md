# Edgerun

Edgerun is a local-first runtime and control-plane project for running agent workflows with deterministic system behavior, explicit contracts, and durable event history.

This repository is designed around one core idea:

> **AI execution should be inspectable, reproducible, and operationally reliable — not opaque.**

---

## Base Philosophy

Edgerun prioritizes:

- **Determinism over convenience**  
  Execution paths, protocol behavior, and state transitions should be explicit and reproducible.

- **Contract-first design**  
  Interfaces between components are defined through typed contracts (proto/schema/message envelopes), not implicit coupling.

- **Event-native architecture**  
  System state is represented through events and timelines so behavior can be queried, replayed, and audited.

- **Local-first control plane**  
  Core operation should work through local bridge endpoints and local infrastructure, with cloud paths as optional extensions.

- **Proof-driven operations**  
  Changes are expected to be validated by commands/tests and not treated as complete by assertion alone.

---

## What This Project Is

At a high level, Edgerun combines:

- a **runtime layer** for deterministic execution,
- a **scheduler/worker control layer** for orchestrating work,
- a **storage/event layer** for durable timelines and event streams,
- a **frontend/control surface** for operators,
- and **deployment/tooling glue** for local stacks and production-style workflows.

---

## Project Parts

### `crates/` (Rust workspace, core system)

Primary system components live here, including:

- runtime execution,
- scheduler and worker processes,
- node manager and local bridge control surfaces,
- event bus and storage foundations,
- protocol/transport/core type crates.

This is the canonical systems core of Edgerun.

---

### `frontend/` (canonical frontend root)

Static-first frontend build and UI surfaces for interacting with Edgerun capabilities.

Includes:

- control and runtime-oriented pages,
- intent/eventbus UI surfaces,
- Cypress E2E coverage and frontend quality gates.

---

### `docs/`

Repository context, execution discipline, protocol documents, and active specs.

The compact canonical docs that define default operating mentality are:

- `docs/REPO_CONTEXT_COMPACT.md`
- `docs/EXECUTION_GUIDELINES_COMPACT.md`

---

### `scripts/`

Operational scripts for:

- local stack bring-up/verification,
- workflow hygiene checks,
- build/deploy helpers,
- environment-specific runtime operations.

---

### `config/`

Configuration templates and runtime policy files (e.g. local gateway/caddy/cloudflared/containerd/storage policy wiring).

---

### `docker-compose*.yml` and `docker/`

Containerized local/system stack definitions and build contexts for core services.

---

### `out/`

All generated, build, and temporary outputs are centralized here to keep the source tree deterministic and clean.

---

## System Model (Conceptual)

Edgerun can be viewed as layered boundaries:

1. **Control plane** (node manager + scheduler interfaces)
2. **Execution plane** (runtime + worker loop)
3. **Event/state plane** (event bus + timeline/storage)
4. **Operator plane** (frontend + local tooling)

A key design goal is to keep these boundaries explicit so system behavior remains understandable as complexity grows.

---

## Long-Term Direction

Edgerun aims to become a dependable execution substrate for agentic workflows where:

- system behavior is observable,
- protocol drift is minimized,
- operational failures are diagnosable,
- and architecture remains disciplined under scale.

In short: **ship fast when possible, but never at the cost of reliability foundations.**
