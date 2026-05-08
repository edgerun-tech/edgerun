# EdgeRun Platform

Shared frontend platform layer for protocol access, routing, node state, app registry, capability registry, permissions/auth tracking, external connections, and package/runtime status.

## Architecture Principles

### Protocol Types
- **Generated protobuf types are the canonical protocol types.**
- Do not create duplicate protocol interfaces for `AppPackage`, `ObjectRef`, `CapabilityGrant`, `CapabilityDescriptor`, `CommandEnvelope`, `AppPrincipal`, `DelegationRecord`, etc.
- UI-specific types are allowed only as view models or drafts.
- Browser app-store package and catalog payloads are an exception to the protobuf
  frontend path: they are internal Edgerun wire records and must cross the
  browser boundary as rkyv decoded by `platform/runtime/edgerun-node.ts`.

### Single Sources of Truth
- **One capability registry** - `platform/registries/capability-registry.ts`
- **One app registry** - `platform/registries/app-registry.ts`
- **One connection/auth tracker** - `platform/state/connection-store.ts` + `platform/auth/external-connection-tracker.ts`
- **One wasm registry** - `platform/runtime/wasm-registry.ts`
- **One router** - `platform/router/edgerun-router.ts`

### Component Responsibilities
- **Components render only.** No protocol logic, no data fetching, no state management.
- **Pages compose only.** Use platform hooks, present UI.
- **Hooks provide access.** All data/actions through `platform/ui/` hooks.
- **Stores own state.** Nanostores in `platform/state/`.
- **Registries own lookup/normalization.** In `platform/registries/`.
- **Trackers own auth/permission lifecycle.** In `platform/auth/`.

### Data Flow
```
Node protocol data
  → protocol client
  → generated protobuf decode
  → platform store
  → selector/hook
  → UI component
  → user action
  → platform command builder
  → generated protobuf command
  → node
```

No bypasses.

## Directory Structure

```
platform/
  protocol/           # Low-level protocol communication
    client.ts         # Single protocol client
    codec.ts          # Protobuf encode/decode, hashing
    refs.ts           # ObjectRef, EventRef, CommandRef parsing/formatting
    commands.ts       # Command building, signing, sending
    objects.ts        # Object fetching, metadata
    apps.ts           # AppPackage, AppPrincipal protocol
    capabilities.ts   # Capability protocol messages
    streams.ts        # Stream events, verification
    errors.ts         # Protocol error types

  state/              # Nanostores for state management
    node-store.ts     # Node identity, health, stream head
    app-store.ts      # Installed apps, grants
    capability-store.ts  # Capabilities, grants, delegations
    connection-store.ts # External connections
    permission-store.ts # User/session permissions
    route-store.ts    # Route state
    object-store.ts   # Object metadata, content cache
    command-store.ts  # Command state, history
    runtime-store.ts  # Runtime state, WASM cache

  registries/         # Lookup/normalization registries
    capability-registry.ts   # Capability descriptors, grants
    app-registry.ts         # App packages, routes, actions, pipelines
    route-registry.ts       # Dashboard and app routes
    component-registry.ts   # UI components, ViewSpec rendering
    connection-registry.ts   # Connection lookup
    tool-registry.ts        # MCP-style tools

  auth/               # Auth and permission tracking
    permission-tracker.ts      # User/session permissions
    external-connection-tracker.ts  # External auth state
    session-tracker.ts       # UI session state
    approval-tracker.ts      # Pending approvals

  router/             # Application routing
    edgerun-router.ts     # Single app-aware router
    route-types.ts         # Route definitions
    route-resolver.ts      # Route matching, params
    route-guards.ts        # Capability/permission guards

  runtime/            # WASM and app runtime
    wasm-registry.ts      # WASM module cache, resolution
    app-runtime.ts        # App lifecycle, execution
    preview-runtime.ts    # App Studio preview
    capability-resolver.ts # Capability satisfaction planning

  ui/                # React hooks and providers
    PlatformProvider.tsx   # Top-level context provider
    useNode.ts            # Node state/actions
    useApps.ts            # App state/actions
    useCapabilities.ts     # Capability state/actions
    useConnections.ts     # Connection state/actions
    usePermissions.ts     # Permission state/actions
    useEdgeRunRouter.ts   # Router access
    useObjects.ts         # Object state/actions
    useCommands.ts        # Command state/actions
    useRuntime.ts         # Runtime state/actions
```

## Rules

1. **No duplicate protocol models.** Use generated protobuf types.
2. **Components do not manually fetch protocol resources.** Use hooks.
3. **Components do not manually parse ObjectRef/CommandRef/EventRef.** Use `platform/protocol/refs.ts`.
4. **Components do not manually decide capability satisfaction.** Use `platform/runtime/capability-resolver.ts`.
5. **State-changing actions become commands/approvals.** Through `platform/protocol/commands.ts`.
6. **No parallel implementations.** New features (MCP, App Studio) must use platform services.

## Browser App Catalog

The browser app catalog is loaded from `public/apps/catalog.ecat`, not from a
JSON catalog. `platform/runtime/browser-app-install-store.ts` fetches that rkyv
file, passes it into `platform/runtime/edgerun-node.ts`, and accepts the decoded
catalog only when the node verifies the catalog signature and the embedded
`storeId` matches `NEXT_PUBLIC_EDGERUN_APP_STORE_ID`.

`public/apps/catalog.json` is only a source index for the publisher. It uses
`edgerun-app-catalog-source-v1` and lists app slugs plus capability ids. It is
not loaded by the browser runtime.

To refresh the published catalog:

```bash
cd frontend
EDGERUN_APP_STORE_SEED=<32-byte-store-seed-hex> \
NEXT_PUBLIC_EDGERUN_APP_STORE_ID=<matching-32-byte-store-public-key-hex> \
node scripts/refresh-app-catalog.mjs
```

The refresh script preflights listed apps before invoking Cargo. It rejects
missing packages, duplicate or unsafe slugs, and JSON `app.edapp` files. Publish
only apps with rkyv `app.edapp` manifests. Do not commit real store seeds.

## Using Platform Services

### In a Component
```tsx
function AppsPage() {
  const { apps, refreshApps } = useApps()
  const { listGrantsForApp } = useCapabilities()

  return <AppList apps={apps} getGrants={listGrantsForApp} onRefresh={refreshApps} />
}
```

### Not Acceptable
```tsx
function AppsPage() {
  fetch("/api/apps")  // ❌ Don't do this
  parse protobuf refs manually  // ❌ Don't do this
  calculate capability grants locally  // ❌ Don't do this
  render everything inline  // ❌ Don't do this
}
```

## Adding New Features

### MCP Control App should use:
- `platform/protocol/client.ts`
- `platform/protocol/commands.ts`
- `platform/registries/capability-registry.ts`
- `platform/auth/permission-tracker.ts`
- `platform/auth/approval-tracker.ts`
- `platform/registries/app-registry.ts`
- `platform/registries/tool-registry.ts`

### App Studio should use:
- `platform/registries/component-registry.ts`
- `platform/protocol/apps.ts`
- `platform/registries/app-registry.ts`
- `platform/registries/capability-registry.ts`
- `platform/auth/permission-tracker.ts`
- `platform/router/edgerun-router.ts`
- `platform/runtime/preview-runtime.ts`
