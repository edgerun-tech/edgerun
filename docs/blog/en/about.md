---
title: About Edgerun
summary: Onboarding overview, architecture, and the operating philosophy behind this platform.
---
# About Edgerun

Edgerun started from a practical constraint: a platform should run from one
source checkout, with a clear trust boundary and one coherent operating model.

If you are onboarding, use this mental model:

- Event logs carry authority.
- Commands are only authoritative after the target node validates and commits
  them.
- Capabilities and policy determine who can do what.
- All operator surfaces (build log, git, mail, status, apps) share the same
  trust root.

## What Edgerun is about

Edgerun is not a separate admin layer or a random CMS. It is an attempt to keep
the architecture aligned across what users can see, what services run, and what
the node accepts as real.

If a claim appears in this build log, it should map to either config, code, or a
committed operational artifact from the running system.

## Core architecture

The current model is identity-first and event-first:

- single-writer streams are append-only, contiguous, hash-linked, and signed;
- commands are authoritative only after the target node validates and records
  commit/reject events;
- objects are immutable, while derived state lives in indexes and snapshots;
- capability delegation is explicit and attenuated;
- access is query-based and policy-checked;
- networking routes through identity.

This model is already serving real workloads in this deployment.

## What is real today

- `edgerun-server` runs production email (SMTP/IMAP/SMTPS/IMAPS), DNS, HTTPS,
  and the runtime web surfaces.
- the build log (this page), source surface, and mail surface are published from
  the same server process.
- the dashboard and build-log surfaces publish checked-in operational state from
  the same deployment boundary.

## What new avenues this opens up

- one dashboard to browse service status, code and blog surfaces,
- a unified transparency path between claims, evidence, and configuration,
- simpler app integration by using browser app surfaces and capability descriptors,
- and a path for operators to own runtime and content from one place.

## For people reading this first

Start with:

1. [Edgerun onboarding](/surface/blog/posts/edgerun-onboarding.html)
2. [Build your first Edgerun app](/surface/blog/posts/build-your-own-edgerun-app.html)
3. [Latest posts and release notes](/surface/blog)

## Philosophy

1. Make trust and authority explicit in data.
2. Keep operational complexity inside the stack boundary.
3. Keep the path from claim to code short.
4. Prefer reproducibility over convenience when the platform claims matter.

Contact is still by email at
[ken@edgerun.tech](mailto:ken@edgerun.tech).

The blog and content are meant to stay grounded: every post should be tied to
actual checked-in code paths, live state, and real deployment behavior.
