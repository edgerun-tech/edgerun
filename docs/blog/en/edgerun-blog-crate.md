---
title: edgerun-blog: publishing without a separate CMS
date: 2026-04-30
author: Ken
summary: The small first-party blog crate behind the Edgerun build log, and why it lives inside the same server as mail and code.
tags: [edgerun, crates, blog, email]
---
# edgerun-blog: publishing without a separate CMS

The build log you are reading is not backed by a hosted CMS, a database, or a
static-site SaaS. It is served by `edgerun-blog`, a small host-only crate that
reads Markdown from the same Git checkout as the code.

This is one of the platform pieces currently powering the Edgerun email
deployment. It is deliberately modest: publish notes, render posts, expose a
feed, and let the dashboard embed the same content as a workspace surface.

## What It Does

Implemented in code today:

- reads Markdown and HTML posts from `docs/blog`;
- requires matching English, Thai, and Estonian content for public posts;
- validates front matter for title, date, author, summary, and tags;
- renders post pages, index pages, feeds, search metadata, sitemap, robots, and app shell assets;
- can serve directly from a checkout or generate deterministic static output;
- links visible crate names to the public code explorer when those crates have been released.

The important part is not that this replaces every feature of a mature CMS. It
does not. The point is that the production mail host can publish release notes
without pulling in a database, plugin runtime, admin panel, or external
analytics script.

## How It Compares

Compared with a general static-site generator, `edgerun-blog` is narrower. It
does not try to be a theme ecosystem. The layout, feed, metadata, language
rules, and dashboard fragments are built for one publishing workflow.

Compared with a database-backed CMS, it has less editorial machinery and less
runtime state. The source of truth is the Git checkout. Review, rollback, and
deployment use the same path as the code.

Compared with a hand-written page, it keeps the repetitive safety work in code:
front matter checks, escaping, feeds, sitemap entries, canonical URLs, and
language routes.

That tradeoff fits the current release strategy. We can publish enough to make
the project understandable without opening a large operational surface just to
host a few pages.

## Why It Matters For Email

Email servers need more than SMTP and IMAP. A real deployment also needs
documentation, status notes, DNS records, webmail, source links, and a way for
operators to understand what changed.

`edgerun-blog` keeps that publishing path inside the same deployment shape as
the mail server. It is one binary serving mail, webmail, build notes, and the
dashboard, with the public code surface released separately through explicit
visibility markers.

That is the pattern we are using for Edgerun releases: expose the operational
pieces that are already carrying production traffic, keep the claims narrow,
and make each public slice easy to inspect.
