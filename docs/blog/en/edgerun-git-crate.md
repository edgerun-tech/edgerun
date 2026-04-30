---
title: edgerun-git: controlled source release from the same checkout
date: 2026-04-30
author: Ken
summary: Why Edgerun uses a constrained, metadata-driven source surface instead of exposing a full forge.
tags: [edgerun, crates, git, release]
---
# edgerun-git: controlled source release from the same checkout

`edgerun-git` is the public code surface behind Edgerun’s release model. It is
not a replacement forge. It is a controlled publish boundary.

The platform’s claim has always been straightforward: only explicitly released
surface should be browsable. That avoids turning the repository into a public
dump and keeps audit scope bounded.

## Implementation currently in production

- repositories and crates are hidden unless release metadata marks them visible,
- file and directory visibility follows `.gitvisible` markers,
- crate metadata is generated from controlled `.edgerun/git/crates/*.txt`,
- crate/API/call-edge/test summaries are shown through the same visibility rules,
- blog and source surfaces share a common policy.

## Why this is architecturally important

The crate couples documentation and code availability. A claim made in the log can
link to a crate route that is already covered by visibility policy. That keeps:

- access checks explicit,
- provenance visible,
- surface expansion deliberate.

For now the public slice is intentionally small, but the mechanism is stable:
release controls first, feature breadth later.

## Current achievement

The production path for this platform already serves its own operational code context
through the same architecture, not a secondary portal or third-party forge.
