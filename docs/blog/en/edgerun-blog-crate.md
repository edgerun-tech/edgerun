---
title: edgerun-blog: publishing without a separate CMS
date: 2026-04-30
author: Ken
summary: How Edgerun publishes operational notes from the same graph, with constrained tooling and real code-backed release metadata.
tags: [edgerun, crates, architecture, email]
---
# edgerun-blog: publishing without a separate CMS

`edgerun-blog` exists because a production platform should not need a separate
editor stack to publish build notes.

It reads Markdown from the same source tree and produces both static and runtime
surfaces inside the same deployment envelope. That keeps publication coupled to
the actual machine state.

## Why this crate exists in architecture terms

Edgerun separates protocol, storage, and presentation, but `edgerun-blog` is the
small “documentation surface” at the end of that chain. It is part of the same
idea as the rest of the platform:

- no separate database-backed CMS,
- deterministic output from checked-in content,
- front matter validated as structured input,
- explicit surfaces for locale/route outputs.

## What is implemented in code today

- Markdown parsing with front matter checks for title, date, author, summary, and tags.
- feed, sitemap, robots, and canonical path output.
- multilingual post discovery with shared rendering pipeline for pages and posts.
- trusted-html pass-through for benchmark charts and code references.
- dashboard integration so the same content appears in the runtime surface.
- controlled link extraction for release-visible crates, enabling claim-backed source
  references.

## How it contributes to the platform

The crate is intentionally narrow. It does not compete with a full CMS, and it
does not try to replace source control.

What it does do is reduce deployment overhead: a single checkout can generate the
same evidence and release notes that an operator already expects from a separate
website system, while keeping the architecture surface unchanged.

## Why this matters

When mail, web, DNS, and code surfaces are all part of one runtime stack, publishing
operational transparency must happen in that same stack too. `edgerun-blog` is the
evidence surface that keeps operational claims close to code reality.
