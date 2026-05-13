# EdgeRun UI Architecture

EdgeRun UI has one shared source of truth: Rust builds `GpuScene` command buffers.
Hosts only render those buffers and forward input back into Rust.

## Layers

1. EdgeRun Shell
   - Owns system UI: launcher, trust/status indicators, capability prompts,
     lock flow, notifications, and future app switching.
   - May be rendered as a separate overlay scene/canvas by web hosts.
   - May be composited into one scene by native hosts.

2. Workspace
   - Owns tiled app placement.
   - Apps do not float over each other.
   - Apps do not own global shell chrome.

3. App Surfaces
   - Render app content into assigned workspace bounds.
   - Use shared UI nodes/components.
   - Should receive projected state from real stores, not hardcoded demo data.

## Host Rules

- JS is a byte bridge only.
- Browser hosts render the workspace canvas plus the shell overlay canvas.
- Native hosts can render a combined scene, but must use the same `UiShellState`
  and `UiWorkspace` action flow.
- Hosts must not know app metadata beyond calling exported Rust functions.
- Hosts must not implement layout, app launching, or trust/capability semantics.

## App Registry

App metadata lives in `EDGERUN_APP_REGISTRY`:

- stable app id;
- `UiAppKind`;
- title;
- launcher detail;
- canonical icon;
- launcher hit id;
- placement: workspace, full-screen system, or example.

Shell launcher rows, app opening, default workspace construction, and host
defaults should use this registry instead of duplicating ids or titles.

## Core Modules

- `src/gpu.rs`: immediate-mode UI node builder, app renderers, and shared
  composition entry points.
- `src/gpu/scene.rs`: foundational scene buffer types, clipping, software
  fallback text emission, and color-scheme remapping.
- `src/gpu/bitmap_font.rs`: compact 5x7 fallback glyph table for no-font
  scene text.
- `src/gpu/primitives.rs`: shared UI geometry, control style, and unified
  chat/contact state structs.
- `src/gpu/node.rs`: JSX-like immediate-mode UI node tree, builder helpers,
  and node rendering dispatch.
- `src/gpu/paint.rs`: shared text measurement/truncation, labels, pills,
  message bubbles, and panel/card drawing helpers.
- `src/gpu/painter.rs`: `UiPainter`, the ergonomic drawing facade used by
  nodes, apps, shell, workspace, and component renderers.
- `src/gpu/apps.rs`: public scene-build entry points and built-in app surface
  renderers for chat, Trust Manager, Storage, lock, capability, and gallery.
- `src/gpu/app_registry.rs`: canonical app ids, launcher ids, app metadata, and
  app surface construction.
- `src/gpu/workspace.rs`: tiled workspace model, app surfaces, tabs, focus, and
  app event routing.
- `src/gpu/shell.rs`: shell state, launcher actions, and shell overlay drawing.
- `src/gpu/text.rs`: font atlas construction, glyph metrics, and text quad
  emission.
- `src/gpu/icons.rs`: canonical icon names, provider mapping, SVG atlas
  metadata, and fallback GPU icon geometry.
- `src/gpu/palette.rs`: shared EdgeRun GPU color constants.
- `src/gpu/webgl2.rs`: browser WebGL2 shader source exports.
- `src/gpu/gl.rs`: native OpenGL renderer backend behind the `gpu-gl`
  feature.
- `src/gpu/runtime.rs`: hit targets, input events, UI actions, focus state,
  scroll state, text input, and runtime control values.
- `src/gpu/components.rs`: reusable dashboard/form/domain primitives built on
  the shared GPU UI node model.
- `src/gpu/style.rs`: Tailwind-like class parser and shared color/style
  resolution.
- `src/gpu/theme.rs`: user-owned style authority, author vision presets, and
  component preview styling state.

## Scene ABI

Renderer-facing data should be packed buffers:

- rect buffer;
- text vertex buffer;
- hit buffer;
- font/icon atlas bytes and UV metadata.

Scalar per-field exports are legacy compatibility and should be removed after
all hosts use packed buffers.

## Current Cleanup Queue

- Split `gpu.rs` into focused modules.
- Move web rendering setup out of the EdgeRun frontend HTML into a minimal reusable host.
- Delete or migrate old pixel-buffer UI paths once GPU UI covers their use cases.
- Keep one maintained native preview binary.
- Replace preview strings with real projected state.
- Add screenshot smoke tests for shell, launcher, full-screen system apps, and
  component gallery.
