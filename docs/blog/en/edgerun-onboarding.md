---
title: Edgerun Onboarding Guide
date: 2026-04-30
author: Ken
summary: A practical first run guide for understanding Edgerun, moving through the live surfaces, and choosing your first task.
tags: [edgerun, onboarding, architecture, usage]
---
# Edgerun Onboarding Guide

This is the fastest route to understand what this platform is doing right now.

If you want to move from curiosity to use quickly:

1. Read one architecture post in the same surface.
2. Open the runtime surfaces from the dashboard.
3. Decide what you want to extend and use the app guide.

## 1) What Edgerun is in one sentence

Edgerun is an identity-first infrastructure runtime where command authority,
stream provenance, and capability policy are explicit.

In practice, this means:

- commands are accepted only when the target node commits them,
- claims should map to config + logs + code, and
- one server process can expose multiple production-grade surfaces.

## 2) What already works in this build

The checkout you are reading from is currently used for a working integrated
surface:

- build log posts from this same checkout,
- source browsing for visible release slices,
- mail and dashboard surfaces,
- global status and chat,
- and Wasm app launch hooks in the dashboard workspace.

If a surface is reachable, it is there because this runtime is serving it.

## 3) Use the platform right now

Start here from this same deployment:

- Open the **Build Log** surface and read the latest entries.
- Open the **Code** surface and inspect public crates.
- Open **Apps** and check installed app modules.
- Use the bottom status area to watch live health.

If you run your own node or test environment, check server flags first:

```bash
cargo run -p edgerun-server -- --help
cargo run -p edgerun-server -- --check-config --config /path/to/server.yaml
```

Those commands are safe and help validate whether your config can start.

## 4) New avenues the model unlocks

The same architecture supports different operator use-cases with one model:

- **Proof-first documentation**: release notes and operational surfaces in one
  trusted context.
- **Unified operations**: DNS, email, HTTPS, and platform surfaces from one runtime
  pattern.
- **Composable apps**: browser modules can live beside core services.
- **Capability-aware interfaces**: explicit permissions instead of hidden
  integrations.

## 5) Where to go next

If this sounds useful, the next step is to build your own app:

- [Build your first Edgerun app](/surface/blog/posts/build-your-own-edgerun-app.html)

After that, compare against the output and commit path in your own config so
your runtime behavior is traceable.

