---
title: About Edgerun
summary: Why Edgerun exists, the architecture we are building, and what is already running in production.
---
# About Edgerun

Edgerun started from a practical constraint: we wanted a production service
running from a single Git checkout, with a small operational surface and clear
boundaries of trust.

Edgerun is not trying to be “more abstract.” It is trying to make every
abstraction explicit: what is stored, who can issue commands, and why a node
accepted a command as authoritative.

## Core architecture

The current model is identity-first and event-first:

- single-writer streams are append-only, contiguous, hash-linked, and signed;
- commands are authoritative only after the target node validates and records
  commit/reject events;
- objects are immutable, while derived state lives in indexes and snapshots;
- capability delegation is explicit and always attenuated so children cannot expand
  parent privilege;
- access is query-based and policy-checked;
- networking routes through identity.

This gives us a useful property: if a node can replay its event stream, it can
reconstruct expected state without manually maintained mutable “truth” fields.

## What is real today

We are dogfooding this architecture now:

- `edgerun-server` runs production email (SMTP/IMAP/SMTPS/IMAPS), DNS, HTTPS,
  webmail, and hosted surfaces.
- the build log and code surface are published through the same operational model
  instead of a separate CMS stack;
- `edgerun-blog` publishes notes from Markdown in this repository and shares
  metadata with the runtime surfaces;
- `edgerun-git` exposes a controlled public slice of source code and crate
  metadata.

The claim is not feature-completeness. The claim is that core pieces are already
carrying real deployment traffic.

## Philosophy

1. Make trust and authority explicit in data, not implicit in tooling.
2. Keep operational complexity inside the stack boundary, not in hidden
   management platforms.
3. Keep the path from claim to code short: if a behavior is real, the source should
   be reachable from the same deployment context.
4. Prioritise reproducibility and recovery over short-term convenience.

In practice that means fewer ad hoc integrations, fewer opaque dependencies, and
more predictable runtime behavior.

I still share earlier exploratory notes here, because this remains iterative:

https://www.youtube.com/watch?v=AIMdIAoiR80

Contact is by email at [ken@edgerun.tech](mailto:ken@edgerun.tech).

The blog documents pieces as they become public. The code explorer is wired to
expose those same pieces so posts can link to real commits and real files instead
of abstract claims.
