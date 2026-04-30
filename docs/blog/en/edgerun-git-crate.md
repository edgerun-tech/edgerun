---
title: edgerun-git: controlled source release from the same checkout
date: 2026-04-30
author: Ken
summary: The Git and crate explorer behind the public Edgerun code surface, built for selective release rather than dumping the whole platform at once.
tags: [edgerun, crates, git, email]
---
# edgerun-git: controlled source release from the same checkout

`edgerun-git` is the small Git and crate explorer behind the public code
surface. It is intentionally not a full forge. There are no issues, pull
requests, accounts, CI pages, or social features.

That is by design. The current goal is controlled release: show the pieces that
explain what is running in production, especially the email-facing platform
code, without exposing every internal or experimental area at once.

## The Visibility Model

Implemented in code today:

- repositories are hidden unless `.edgerun/git.yaml` says `visible: true`;
- files and directories are hidden unless released with `.gitvisible` markers;
- generated crate metadata is read from `.edgerun/git/crates/*.txt`;
- the dashboard can render repository, crate, API, call-edge, test, and related RFC summaries from that released surface;
- the raw source tree and generated crate explorer share the same visibility policy.

Right now the public slice is intentionally small: the blog crate, the Git
crate, and the blog content. That is enough to explain the build log and the
source explorer themselves without publishing unrelated platform internals.

## How It Compares

Compared with `cgit`-style source browsing, `edgerun-git` is more opinionated
about release boundaries. It is not just "point at a repository and browse
everything." Visibility is part of the repository.

Compared with a full forge, it is much smaller. There is no project management
surface and no account system to operate. For this deployment, those features
would be extra moving parts around a simple need: let readers inspect the code
behind a public claim.

Compared with hosted code pages, it keeps source publication on the same
machine and under the same operational model as the email server. That matters
for Edgerun because the story is not only the code; it is the ability to run the
mail, web, blog, and code surfaces together without a large stack behind them.

## Why It Matters For Email

The email benchmark post makes claims about a real server. `edgerun-git` gives
those claims a controlled inspection path. When a crate is public, the blog can
link directly to that crate in the dashboard and show the visible API and call
shape without turning the whole repository into a product launch.

That lets us release the platform gradually. First publish the parts that are
already useful and understandable. Then widen the surface when the next feature
has evidence, documentation, and a clear boundary.
