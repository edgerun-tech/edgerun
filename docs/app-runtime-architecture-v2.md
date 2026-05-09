# EdgeRun App Runtime Architecture v2

## Goals

- Locking or sealing a Trust Container must remove every live third-party API session.
- A profile must never imply that app API sessions are live.
- Apps must run with explicit capabilities and app-scoped storage.
- Apps must interoperate through `edgerun-node`, not direct shared mutable UI state.
- Browser performance should favor lazy loading, durable indexed storage, and short-lived session material.

## State Boundaries

### Profile State

The profile contains identity, trust roots, contacts, app install records, and sealed app storage indexes. It does not contain live API sessions.

Profile writes must validate the whole profile before sealing. Invalid app metadata is dropped or rejected before encryption.

### App Install State

An installed app is identified by SDK package identity:

- `app_id`
- `release_id`
- `developer_id`
- `manifest_sha256`
- `code_sha256`
- declared routes
- storage namespaces
- provided capabilities
- required capabilities

The SDK model is the source of truth for install records. Builtin apps should be represented as first-party signed packages or compatibility shims that produce equivalent install records.

### App Secret State

Secrets are app-scoped, not profile-global OAuth blobs.

Each secret is stored under:

```text
profile.appStorage[app_id][namespace]
```

The sealed object includes:

- `app_id`
- `release_id` when version-bound
- `secret_kind`
- provider account metadata
- encrypted credential payload
- validation timestamp
- capability scope that allowed the secret to be created

Live tokens must not be readable by other apps. Sharing requires an explicit capability-mediated export.

### API Session State

API sessions are volatile. They live in the app-session broker, keyed by profile, app identity, and capability grant. They must be cleared when:

- profile locks
- profile switches
- profile imports
- user signs out
- trust container is sealed
- app capability grant is revoked
- app is uninstalled

API session failure only affects that app's session state. It must not affect profile unlock state or app install state.

Provider tokens must not be stored in server cookies. Browser apps and WASM apps request mediated provider calls through the host bridge; the broker attaches a short-lived bearer only to same-origin proxy requests that match the granted app capability. Durable refresh credentials belong only in app-scoped sealed storage.

## Capability Model

Apps request capabilities declared in their SDK manifest. The host grants capability sessions per app, per profile, and per scope.

Capability examples:

- `identity.read.public`
- `identity.sign.intent`
- `storage.namespace.read`
- `storage.namespace.write`
- `network.fetch.scoped`
- `oauth.session.create`
- `oauth.secret.read.self`
- `node.message.send`
- `node.message.subscribe`
- `dns.zone.read`
- `dns.record.write`

Capability grants are records, not boolean UI flags:

```text
grant_id = hash(profile_id, app_id, release_id, capability, scope, issued_at)
```

Each grant includes expiration, revocation status, and audit metadata.

## Sandboxing

Runtime types:

- `builtin-shim`: first-party UI wrapper around a signed app model.
- `browser-iframe`: isolated origin/frame with postMessage host bridge.
- `sandboxed-worker`: worker runtime with denied globals and explicit host calls.
- `wasm`: WASM module loaded through `edgerun-node`.

Apps do not receive raw profile objects, browser cookies, or global connector state. They receive a host bridge bound to their app identity.

## Host Bridge

All app operations go through a request envelope:

```text
{
  app_id,
  release_id,
  profile_id,
  capability,
  scope,
  request_id,
  payload_hash,
  payload
}
```

The host bridge validates:

- app is installed
- package identity matches
- capability was granted
- scope matches request
- request is auditable

Then it routes to either browser host APIs, a service worker session broker, or `edgerun-node`.

## edgerun-node Mediation

`edgerun-node` is the interop and message authority:

- app install validation
- package graph verification
- capability declaration evaluation
- app-to-app message routing
- profile-to-profile encrypted message routing
- node relay route publication
- audit event emission

Apps do not message each other directly. They emit intents or messages to `edgerun-node`; recipients subscribe through node-mediated channels.

## Connector Apps

Gmail, Google Drive, GitHub, Cloudflare, Photos, and Contacts are connector apps.

Their architecture:

1. App is installed from SDK-style package metadata.
2. User grants provider-specific capabilities.
3. OAuth/API token setup creates a volatile API session.
4. User may choose to seal refresh credentials into the app namespace.
5. On lock/switch/sign-out, volatile sessions are cleared.
6. On next unlock, app is available but disconnected until explicitly reconnected or until a user-approved short-lived resume occurs.

Provider cookies are never the authority for whether a profile is safe or unlocked.

## Performance

- Load app packages and WASM lazily.
- Keep verified package metadata in IndexedDB.
- Keep live secrets only in memory or service worker memory.
- Use short-lived one-use resume handles for refresh ergonomics.
- Cache catalog verification results by `catalog_sha256`.
- Avoid global React stores for app internals; app session stores are app-local and profile-keyed.

## Migration Plan

1. Immediate: clear all third-party session cookies and broker sessions on lock/switch/import/sign-out.
2. Move provider session handling behind a common `/api/session/clear` and app-session broker.
3. Replace `profile.appSecrets` with app-scoped sealed storage records.
4. Add app identity fields to every connector secret.
5. Convert builtin connectors into signed first-party app descriptors.
6. Route app host calls through a capability bridge.
7. Route app-to-app and profile messaging through browser `edgerun-node` WASM.
8. Remove remaining display-only connector cookie reads from UI components.
