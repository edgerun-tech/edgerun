# Browser Nodes and Wasm Agents

Status: design note, grounded in existing protocol and crate boundaries.

This document describes a direction for turning `dash.edgerun.tech` into a
browser-hosted Edgerun node. It is not a separate application platform or a new
configuration system. It reuses the v0 protocol model, `edgerun-config`,
`edgerun-capabilities`, `edgerun-remote-capability`, `edgerun-node`, and the
existing web UI shell.

## Protocol Mapping

The core protocol defines a node as an active participant capable of storing
data, establishing sessions, enforcing local policy, or writing streams. A
browser tab or installed browser profile can satisfy that definition.

A browser node is therefore:

- a protocol node with constrained storage and transport,
- an execution environment for Wasm agents,
- a local policy boundary for user-approved capabilities,
- a holder of derived views and local drafts,
- and, once identity support is wired, a writer of one or more browser-owned
  streams.

The browser node is not authoritative for remote resources such as mailbox
delivery, repository visibility, or server configuration. Those remain
authoritative only after the target node validates a command or capability
invocation and records the outcome in its own stream.

## Existing Crate Responsibilities

- `edgerun-config`: configuration resource envelope and typed resources. Browser
  app catalogs and browser node policy should be added here if they need
  declarative configuration.
- `edgerun-capabilities`: capability descriptors, grants, selectors,
  constraints, and validation helpers.
- `edgerun-remote-capability`: session open/accept/invoke/event/close envelope
  model for remote capability providers.
- `edgerun-node`: command, query, store, controller, and stream semantics for
  protocol nodes.
- `edgerun-dash-webapp`: host-rendered shell and bootstrap surface for browser nodes.
- `edgerun-vfs`, `edgerun-virtual-disk`, `edgerun-edgefs`: possible storage
  providers behind file/object capabilities, not ambient filesystem access for
  Wasm agents.

No new parallel manifest/config crate should be introduced for this.

## Wasm Agents

Mail, Code, Build Log, Files, and future tools should be modeled as Wasm agents
running inside the browser node. An agent is not trusted just because it is
loaded by Dash. It receives only the host calls and capability handles granted
to it.

An agent can:

- render UI inside a mount owned by Dash,
- keep local UI state,
- request capabilities,
- query derived views,
- invoke remote capabilities,
- write local browser-node state when granted,
- and receive events from open capability sessions.

An agent must not:

- receive raw credentials,
- access arbitrary network origins,
- access arbitrary user or app storage,
- mutate authoritative remote state directly,
- or escape its mount point into the whole Dash DOM.

## App Catalog as Config

The browser app catalog should be expressed as `edgerun-config` resources, not a
separate package manager format. The exact resource names are still design work,
but the shape should be close to:

```yaml
apiVersion: edgerun.io/v1alpha1
kind: BrowserApp
metadata:
  name: mail
spec:
  app_id: edgerun.mail
  title: Mail
  module:
    url: /apps/mail/app.wasm
    sha256: ...
  surfaces:
    - mail
  required_capabilities:
    - selector: mail://edgerun.tech/*
      operations: [query, read, send]
    - selector: fs://app/edgerun.mail/*
      operations: [query, read, write]
```

This is protocol/design work. The current code does not yet implement
`BrowserApp` as a `ConfigResource`.

## Capability Shape

The browser node should expose capabilities to Wasm agents as opaque handles.
The handles map back to protocol capability grants and remote capability
sessions.

Useful capability families:

- `mail.query`, `mail.read`, `mail.send`, `mail.watch`
- `fs.list`, `fs.read`, `fs.write`, `fs.watch`, `fs.commit`
- `git.query`, `git.read_object`, `git.read_tree`, `git.search`
- `blog.query`, `blog.read_post`
- `search.query`

These names are design placeholders. They should be represented with existing
capability descriptors, selectors, operations, and constraints rather than a new
permission model.

## Browser Storage

Browser storage should be treated as a local node storage tier. IndexedDB can
hold:

- node identity material when software/browser-held keys are acceptable,
- local stream segments,
- derived query results,
- cached immutable objects,
- Wasm module cache metadata,
- drafts and unsent commands,
- and revocation/state checkpoints.

Persistent remote effects still require target-node commitment. A local draft is
not a sent email. A local file edit is not a published object. A local app
install is not a delegated authority unless there is a valid grant.

## Mail as First Browser Agent

Mail is the right first migration target because it exercises the model without
requiring public SEO.

Initial browser-agent mail behavior:

- query inbox summary from the mail/server node,
- read a message by id,
- save compose/reply drafts locally,
- attach files through a file capability,
- invoke `mail.send`,
- display committed or rejected send results,
- and refresh from event/session updates.

The existing server-rendered mail HTML should remain as fallback and as a useful
debugging surface while the Wasm agent matures.

## Dash Runtime Boundary

Dash should own:

- browser node bootstrap,
- app catalog loading from config-derived resources,
- Wasm module loading and hashing checks,
- capability grant prompts and persistence,
- host-call dispatch,
- routing and workspace layout,
- and focus/accessibility behavior after app swaps.

Agents should own:

- their local UI,
- local component state,
- capability invocation requests,
- and rendering of data returned through granted handles.

## Current Status

Implemented in code today:

- `edgerun-dash-webapp` can embed a JSON list of workspace modules in the page shell.
- Dash has native HTML surfaces for Build Log, Code, and Mail.
- `edgerun-capabilities` and `edgerun-remote-capability` provide protocol
  building blocks for descriptors, grants, sessions, and invocation envelopes.
- `edgerun-config` already owns K8s-style resource parsing and typed resources.
- `edgerun-vfs`, `edgerun-virtual-disk`, and `edgerun-edgefs` exist as storage
  and filesystem-related crates.

Protocol/design requirements:

- browser node identity lifecycle,
- browser-node stream storage,
- config resources for browser app catalogs and browser node policy,
- Wasm host-call ABI,
- capability handle mapping,
- remote capability transport over browser-friendly channels,
- and mail/code/file agents using those handles.

Not implemented yet:

- `BrowserApp` or equivalent config resource,
- signed app catalog validation,
- Wasm agent host-call ABI,
- browser node IndexedDB stream/object store,
- remote capability bridge in Dash,
- or Mail as a Wasm agent.

## Implementation Sequence

1. Add design-aligned config resources to `edgerun-config` for browser apps and
   browser node policy.
2. Replace hardcoded Dash `WorkspaceModule` data with config-derived catalog
   output.
3. Define the minimal Wasm host-call ABI in `edgerun-dash-webapp` docs and bootstrap
   code.
4. Add a browser-friendly transport bridge for `edgerun-remote-capability`
   envelopes.
5. Convert Mail into the first Wasm agent while keeping server-rendered fallback.
6. Add browser local storage for drafts, cached views, and eventually
   browser-node streams.
7. Move Code/File interactions onto the same capability path.

The guiding rule is simple: Dash hosts a browser node; Wasm modules are agents
inside that node; authority remains explicit, delegated, and committed by the
target node that owns the state.
