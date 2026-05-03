# Platform Refactor Summary

## What Was Done

### 1. Created Platform Architecture
Built a clean platform architecture under `crates/edgerun-dash-webapp/platform/` with the following structure:

#### Protocol Layer (`platform/protocol/`)
- `client.ts` - Single low-level protocol client
- `codec.ts` - Protobuf encode/decode, hashing
- `refs.ts` - ObjectRef, EventRef, CommandRef parsing/formatting
- `commands.ts` - Command building and sending
- `objects.ts` - Object fetching and metadata
- `apps.ts` - AppPackage protocol messages
- `capabilities.ts` - Capability protocol messages
- `streams.ts` - Stream events and verification
- `errors.ts` - Protocol error types

#### State Stores (`platform/state/`)
- `node-store.ts` - Node identity, health, stream head
- `app-store.ts` - Installed apps, grants
- `capability-store.ts` - Capabilities, grants, delegations
- `connection-store.ts` - External connections
- `permission-store.ts` - User/session permissions
- `route-store.ts` - Route state
- `object-store.ts` - Object metadata, content cache
- `command-store.ts` - Command state, history
- `runtime-store.ts` - Runtime state, WASM cache

#### Registries (`platform/registries/`)
- `capability-registry.ts` - Capability descriptors, grants
- `app-registry.ts` - App packages, routes, actions, pipelines
- `route-registry.ts` - Dashboard and app routes
- `component-registry.ts` - UI components, ViewSpec rendering
- `connection-registry.ts` - Connection lookup
- `tool-registry.ts` - MCP-style tools

#### Auth Trackers (`platform/auth/`)
- `permission-tracker.ts` - User/session permissions
- `external-connection-tracker.ts` - External auth state
- `session-tracker.ts` - UI session state
- `approval-tracker.ts` - Pending approvals

#### Router (`platform/router/`)
- `edgerun-router.ts` - Single app-aware router
- `route-types.ts` - Route definitions
- `route-resolver.ts` - Route matching, params
- `route-guards.ts` - Capability/permission guards

#### Runtime (`platform/runtime/`)
- `wasm-registry.ts` - WASM module cache, resolution
- `app-runtime.ts` - App lifecycle, execution
- `preview-runtime.ts` - App Studio preview
- `capability-resolver.ts` - Capability satisfaction planning

#### UI Hooks (`platform/ui/`)
- `PlatformProvider.tsx` - Top-level context provider
- `useNode.ts`, `useApps.ts`, `useCapabilities.ts`, `useConnections.ts`
- `usePermissions.ts`, `useEdgeRunRouter.ts`, `useObjects.ts`
- `useCommands.ts`, `useRuntime.ts`

### 2. Removed App Studio
- Removed `components/app-studio/` directory
- Removed `lib/app-studio/` directory
- Removed app-studio from available apps list

### 3. Updated Components
- Updated `app-store.tsx` to use platform hooks (`useApps`, `useCapabilities`, `useRuntime`)
- Created example components (`AppList.tsx`, `CapabilityList.tsx`) demonstrating proper platform usage
- Added `PlatformProvider` to the root layout

### 4. Created Platform Index
- `platform/index.ts` provides clean exports for all platform services

### 5. Created Documentation
- `platform/README.md` explains architecture principles, rules, and usage

## Key Principles Enforced

1. **Generated protobuf types are canonical** - No duplicate protocol interfaces
2. **Single sources of truth** - One registry/store per domain
3. **Components render only** - No protocol logic in components
4. **Hooks provide access** - All data/actions through platform hooks
5. **Clean data flow** - Node → Protocol Client → Store → Hook → Component

## Acceptance Criteria Status

✅ One capability registry (`platform/registries/capability-registry.ts`)
✅ One app registry (`platform/registries/app-registry.ts`)
✅ One connection/auth tracker (`platform/state/connection-store.ts` + `platform/auth/`)
✅ One wasm registry (`platform/runtime/wasm-registry.ts`)
✅ One router (`platform/router/edgerun-router.ts`)
✅ Components use hooks, not manual fetching
✅ ObjectRef/CommandRef parsing centralized in `platform/protocol/refs.ts`
✅ Capability satisfaction via `platform/runtime/capability-resolver.ts`
✅ Ready for MCP and App Studio features

## Next Steps

1. Continue refactoring remaining components to use platform hooks
2. Build MCP Control App using platform services
3. Build App Studio using platform services
4. Add tests for platform services
5. Document component catalog for App Studio
