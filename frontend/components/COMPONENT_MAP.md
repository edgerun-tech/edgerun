# Component Map

This frontend has several generations of components in the tree. New code should use the canonical groups below instead of importing visually similar legacy or demo components directly.

## Live previewer

Route: `/components`

Implementation:

- `components/previews/component-preview-registry.tsx`
- `components/previews/component-previewer.tsx`
- `app/components/page.tsx`

Add a component to the preview registry before promoting it into production UI. This keeps production, scaffolding, and legacy pieces visible instead of scattered.

## Canonical production groups

### OS shell

Use these for the desktop/app-surface model:

- `components/os/index.ts` — canonical OS component barrel
- `components/os/desktop.tsx` — main desktop entry used by `app/page.tsx`
- `components/os/xray-desktop-surface.tsx` — permanent graph-first desktop stage
- `components/os/app-overlay-host.tsx` — chrome-less temporary app overlay host
- `components/os/top-bar.tsx` — global top status/control bar
- `stores/index.ts` — canonical store barrel
- `stores/desktop-store.ts` — canonical `AppSurface` state
- `stores/app-launcher.tsx` — canonical app launch path
- `platform/registries/index.ts` — canonical registry barrel
- `platform/registries/app-surface-registry.ts` — canonical app presentation metadata

Terminology: use `AppSurface`, not `Window`, for new code.

### App identity rules

Canonical app IDs should be stable product concepts, not old implementation names.

- `finances` is the canonical finance hub app ID.
- `wallet` is a compatibility alias only.
- `people` is the canonical people/contacts/calling/chat hub app ID.
- `contacts`, `calling`, and `chat` are compatibility aliases only unless they become separate apps later.

Use `normalizeAppId()` for installed-app state and `normalizeBuiltinAppId()` for built-in app lookup.

### Xray

Use the TypeScript feature implementation only:

- `features/xray/index.ts`
- `features/xray/XrayWorkspace.tsx`
- `features/xray/XrayViewport.tsx`
- `features/xray/XrayInspector.tsx`
- `features/xray/XrayCommandSurface.tsx`
- `features/xray/graph/*`
- `features/xray/layout/*`
- `features/xray/render/*`
- `features/xray/services/*`

Do not import from `lib/xray/*` in new code. That directory is legacy JS scaffolding.

### App surfaces

These are production or production-candidate app surfaces:

- `components/os/settings-app.tsx`
- `components/os/finances-app.tsx`
- `components/os/trust-manager-app.tsx`
- `components/os/trust-manager-surface.tsx`
- `components/os/trust-manager-workspace.tsx`
- `components/os/terminal.tsx`
- `components/os/file-manager.tsx`
- `components/os/gmail-app.tsx`
- `components/os/chat-app.tsx`
- `components/os/calling-app.tsx`
- `components/os/contacts-app.tsx`
- `components/os/people-app.tsx`
- `components/os/app-store.tsx`
- `components/os/workflow-builder.tsx`

These should gradually be wrapped in shared app-shell/sidebar primitives instead of each inventing its own internal layout.

### Shared UI primitives

Use `components/ui/*` as primitives only. They should not import app state, desktop state, or platform runtime state.

Important primitives for the OS direction:

- `components/ui/sidebar.tsx`
- `components/ui/dialog.tsx`
- `components/ui/sheet.tsx`
- `components/ui/card.tsx`
- `components/ui/button.tsx`
- `components/ui/badge.tsx`
- `components/ui/tabs.tsx`
- `components/ui/select.tsx`
- `components/ui/switch.tsx`
- `components/ui/slider.tsx`
- `components/ui/resizable.tsx`

## Candidate groups

### Observability candidates

These may become pinned widgets or app-surface content, but should not own desktop layout:

- `components/observability/ConkyOverlay.tsx`
- `components/observability/NodeGrid.tsx`
- `components/observability/ObservabilityDashboard.tsx`
- `components/observability/ResourceOverview.tsx`
- `components/os/resource-monitor.tsx`

### AI surface components

`components/ai/*` is a large set of assistant/artifact primitives. Treat it as its own feature surface. Do not mix these into OS layout primitives unless promoted through the preview registry.

### Agent UI experiments

- `components/agents/*`
- `components/agents-ui/*`
- `hooks/agents-ui/*`

These are visual/runtime experiments until wired into a production app surface.

## Legacy / review before using

These names are now ambiguous or superseded:

- `components/os/window.tsx` — old draggable chrome window model
- `components/os/stage-manager.tsx` — old stage-manager window strip model
- `components/os/desktop-telemetry.tsx` — old conky background layer; superseded by `xray-desktop-surface.tsx`
- `components/os/globe.tsx` — older desktop visual; xray is now the primary desktop stage
- `components/os/wallet-app.tsx` — old wallet-specific surface; finance hub is `finances-app.tsx`
- `components/xray/XrayDashboard.tsx` — duplicate xray entrypoint; prefer `features/xray/*`
- `lib/xray/*` — older JS xray renderer/adapter/layout implementation; prefer `features/xray/*`
- `platform/registries/window-registry.ts` — old draggable window metadata; prefer `platform/registries/app-surface-registry.ts`

Do not delete these until imports are verified locally with a build. Move them to a `legacy/` folder only after no production imports remain.

## Demo / scaffold components

These should not be imported by production code unless promoted through the preview registry:

- `components/*-demo*.tsx`
- `components/animated-modal-demo.tsx`
- `components/cards-demo-*.tsx`
- `components/code-block-demo-*.tsx`
- `components/expandable-card-demo-standard.tsx`
- `components/glowing-stars-demo.tsx`
- `components/hero-section-demo-1.tsx`
- `components/layout-grid-demo.tsx`
- `components/moving-border-demo.tsx`
- `components/multi-step-loader-demo.tsx`
- `components/world-map-demo.tsx`

## Migration rule

1. New desktop/app work goes through `AppSurface`.
2. New graph work imports from `features/xray` only.
3. New visual primitives go in `components/ui` only if stateless and reusable.
4. New production app content goes in `components/os` until app features are split into `features/*`.
5. Anything copied from demos must first be added to `/components` previewer and classified.
