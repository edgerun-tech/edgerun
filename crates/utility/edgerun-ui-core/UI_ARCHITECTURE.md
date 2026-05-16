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
   - Owns tiled surface placement.
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
- Hosts must not implement layout or scene construction.
- Production app metadata, app launching, Trust Manager state, and capability
  semantics belong outside `edgerun-ui-core`.

## App Ownership Boundary

`edgerun-ui-core` does not own the EdgeRun app catalog and does not include an
app registry. Core owns scene buffers, layout/workspace primitives, runtime
input handling, components, style tokens, font/icon assets, and host contracts.
Product hosts and app crates provide projected app state, app metadata,
launching, trust semantics, and capability semantics from their own application
layer.

The shadcn showcase stays in core because it is the canonical reusable
component reference. It is not an app registry and must not grow app launching,
app metadata, or product-specific app surfaces.

Application crates expose UI through exactly one canonical boundary:
`UiSurfaceApp`. A surface builds a `UiNode` tree for its assigned viewport and
handles semantic `UiAction` values. SDL and WebGL hosts are instantiated through
UI core helpers such as `instantiate_sdl_app`, `instantiate_webgl_app`, and
`UiSurfaceHost`; app crates must not recreate pointer capture, scroll wheel,
text input, focus, drag/drop, transition ticks, scene packing, or renderer
setup. Apps may receive normalized `UiEvent` values for app-level shortcuts or
projected state changes; raw host events stay behind UI core adapters.

## Core Modules

- `src/gpu.rs`: immediate-mode UI node builder, canonical component builders,
  renderer contracts, and shared composition entry points.
- `src/gpu/app_host.rs`: canonical `UiSurfaceApp` and `UiSurfaceHost` boundary
  for app crates and host/runtime ownership.
- `src/gpu/scene.rs`: foundational scene buffer types, clipping, canonical
  text emission, and color-scheme remapping.
- `src/gpu/spacing.rs`: shared spacing and sizing tokens used by shell,
  workspace, and reusable components.
- `src/gpu/bitmap_font.rs`: compact 5x7 no-font text path for minimal/debug
  scene builds.
- `src/gpu/primitives.rs`: shared UI geometry, control style, and unified
  chat/contact state structs.
- `src/gpu/node.rs`: JSX-like immediate-mode UI node tree, builder helpers,
  and node rendering dispatch.
- `src/gpu/paint.rs`: shared text measurement/truncation, labels, pills,
  message bubbles, and panel/card drawing helpers.
- `src/gpu/painter.rs`: `UiPainter`, the ergonomic drawing facade used by
  nodes, workspace, and component renderers.
- `src/gpu/workspace.rs`: generic tiled workspace model, surface placement,
  tabs, focus, and surface event routing.
- `src/gpu/shell.rs`: shell state and shell-level action flow without an app
  registry.
- `src/gpu/text.rs`: font atlas construction, glyph metrics, and text quad
  emission.
- `src/gpu/icons.rs`: canonical icon names, provider mapping, SVG atlas
  metadata, and deterministic GPU icon geometry for no-atlas builds.
- `src/gpu/palette.rs`: shared EdgeRun GPU color constants.
- `src/gpu/webgl2.rs`: browser WebGL2 shader source exports and packed
  `UiSurfaceHost` instantiation.
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

## Ownership Boundaries

- `scene`, `webgl2`, and `gl` own renderer-facing commands, packed buffers,
  shader/texture contracts, and frame/debug statistics.
- `shell` and `workspace` own generic shell/workspace action flow and tiled
  placement. App metadata, launcher rows, and app opening live outside core.
- `components`, `paint`, `painter`, `style`, and `spacing` own reusable visual
  primitives and product style contracts.
- `runtime`, `accessibility`, and `app_host` own input projection, focus,
  scroll, text buffers, accessibility output, and frame-session state.
- `text`, `icons`, `font`, and generated Tabler modules own asset lookup,
  atlas metadata, glyph layout, and canonical icon mapping.

OS windows, JS canvas setup, SDL event loops, GL context creation, timers, and
filesystem access belong in host helpers or feature-gated runtime modules inside
UI core, then are exposed through canonical instantiation helpers. App surfaces
should receive projected state and emit scene commands; they should not own host
lifecycle, raw event loops, renderer setup, or duplicate app metadata.

## Build Features

| Feature | Scope | Notes |
| --- | --- | --- |
| no default features | Portable core | Keeps the smallest build for no-font/no-atlas consumers. |
| `std` | Host helpers | Enables OS-facing helper APIs without forcing font rendering. |
| `fontdue-text` | Font atlas text | Enables Inter atlas construction, glyph metrics, and text quads. |
| `tabler-svg-atlas` | Atlas icons | Enables generated alpha atlas metadata and icon quads. |
| `gpu-gl` | Native GL | Enables the native OpenGL renderer backend. |
| `sdl` | Preview/input glue | Enables SDL adapters and depends on `gpu-gl` and `fontdue-text`. |

Common verification commands are documented in `README.md`.

## Spacing Contracts

Shared spacing lives in `src/gpu/spacing.rs`. Use those tokens when a value
affects shell chrome, workspace geometry, rows, cards, or reusable controls.
Local constants are acceptable for one-off component detail, but repeated
values should graduate into spacing tokens before more surfaces depend on them.
Component padding density, row icon slots, row icon gaps, row text insets,
control heights, table cells, toolbar controls, and workspace chrome are
central contracts. Components may choose dense/default/spacious padding, but
should not invent separate row rhythm values.

Controls should keep stable dimensions across hover, focus, active, disabled,
loading, and long-label states. Text should truncate or wrap inside assigned
bounds instead of resizing fixed controls.

## Asset Pack Contracts

Icons, fonts, emoji, and component inventories are replaceable only through a
validated pack contract. `src/gpu/assets.rs` defines the deterministic limits
and coverage requirements:

- icon packs must cover every canonical `UiIcon` and stay inside atlas/count
  limits;
- font packs must provide a default face with required UI characters and stay
  inside face/atlas limits;
- emoji packs must provide the semantic emoji keys used by system surfaces;
- component packs must cover every required component kind.

The compiled default asset set is still required to be valid. With
`tabler-svg-atlas` enabled, missing canonical icon atlas entries are hard
failures, not runtime fallbacks. Runtime replacement packs should be validated
before being installed into a host/session so scene construction remains
deterministic.

Run `tools/report_icon_coverage.py` after regenerating icon atlases to inspect
canonical coverage, provider aliases, and unused atlas entries across Tabler and
Lucide.

`EDGERUN_TABLER_INTER_ASSET_PACK` is the default compiled contract.
`EDGERUN_LUCIDE_GEIST_ASSET_PACK` demonstrates a valid runtime replacement
using the vendored Lucide alpha atlas and vendored Geist variable font face.
Hosts can swap from one to the other through `UiAssetPackRuntime::replace`;
invalid replacements are rejected before they become active. The validation
contract remains separate from upload and host selection so replacements can
change at runtime only when they satisfy the same deterministic limits.

## First-Class Layouts

`UiNode` owns reusable layout semantics so hosts do not recreate them:

- `row` and `column` for stack layouts;
- `grid`, `grid_auto`, and `grid_auto_for_width` for row-based responsive
  tracks with column spans;
- `masonry`, `masonry_auto`, and `masonry_auto_for_width` for shortest-column
  packing of variable-height cards;
- `bento_grid`, `bento_grid_auto`, and `bento_grid_auto_for_width` for
  dashboard/product grids where cards may reserve multiple grid cells.

Masonry and bento layouts are core layout primitives, not surface examples. They
resolve through `UiNode::resolve_layout`, render through the same scene path as
other nodes, and participate in layout tracing and issue detection.

## First-Class Interactions

Input is part of the Rust scene/runtime contract. Hosts forward `UiEvent`
values into `UiRuntimeState`; they do not own semantic interpretation of
pointer gestures.

Drag and drop uses the same command-buffer model as hits:

- `UiNode::draggable(scope_id, item_id, index)` emits a `GpuDragSource`;
- `UiNode::drop_target(scope_id, index)` emits a `GpuDropTarget`;
- `UiNode::reorderable(scope_id, item_id, index)` emits both for common list,
  masonry, and bento reordering cases;
- `UiRuntimeState::drag_session()` exposes current drag state for previews and
  overlays;
- runtime actions report `DragStarted`, `DragMoved`, `Dropped`, `Reordered`,
  and `DragCancelled`.

`scope_id` keeps unrelated reorder groups separate. `item_id` identifies the
dragged data item, while `index` describes the source or target slot. Component
or surface state applies the returned `Reordered { scope_id, item_id, from, to }`
action; browser and native hosts only transport the event and redraw the scene.

Transitions are also Rust-owned:

- `UiTransitionSpec` defines the stable transition contract: id, property,
  from/to values, duration, delay, and easing;
- `UiNode::fade`, `UiNode::slide_x`, and `UiNode::slide_y` cover the common
  opacity and movement cases;
- `UiNode::transition` accepts an explicit transition spec for lower-level use;
- rendering records transition specs in `GpuScene`;
- `UiRuntimeState::sync_transitions` starts scene-declared transitions, and
  `advance_transitions(delta_ms)` advances them with Rust easing;
- node rendering applies current transition values to the commands emitted by
  that node, including draw geometry, hits, drag sources, and drop targets.

Hosts may supply frame ticks and redraw requests, but they should not duplicate
transition curves, progress state, or transform hit geometry in JS/native glue.

`UiRuntimeState::handle_frame(scene, UiFrameInput)` is the preferred host
boundary. It syncs scene-declared transitions, advances transition time,
processes ordered input events, and returns `UiFrameOutput` with semantic
actions plus redraw/active-transition signals. Lower-level methods remain
available for focused tests and specialized internal surfaces, but host
integrations should converge on the frame call.

Shell and workspace code follow the same shape:

- `UiShellState::handle_frame` maps batched runtime actions into shell actions;
- `UiWorkspace::handle_frame` advances surface runtime transitions and batches
  workspace/surface actions;
- `UiShellFrameOutput` and `UiWorkspaceFrameOutput` keep redraw and
  active-transition signals visible without exposing host-owned behavior.

## Scene ABI

Current ABI version: `GPU_SCENE_ABI_VERSION = 1`.

Renderer-facing data should be packed buffers:

- rect buffer;
- text vertex buffer;
- icon vertex buffer;
- hit buffer;
- drag source and drop target metadata;
- transition metadata;
- font/icon atlas bytes and UV metadata.

Packed buffer contracts:

- rect entries use `RECT_FLOAT_STRIDE`;
- text vertices use `TEXT_VERTEX_FLOAT_STRIDE`;
- icon vertices use `ICON_VERTEX_FLOAT_STRIDE`;
- hit entries use `HIT_FLOAT_STRIDE`;
- hosts should check `UiRendererCapabilities.scene_abi_version` before assuming
  a renderer can consume a scene.

Scalar per-field exports are legacy compatibility and should be removed after
all hosts use packed buffers.

## Font And Icon Assets

Inter bytes live at `assets/Inter.ttc`; Geist variable bytes live at
`assets/Geist-Variable.ttf`. `fontdue-text` hosts build a `FontAtlas`, upload its
alpha bytes, and render the emitted text quads or packed text vertices. Atlas
overflow is observable through `FontAtlas` so hosts can surface missing coverage
instead of silently losing glyphs.

Canonical icon use starts with `UiIcon`. `tabler-svg-atlas` and
`lucide-svg-atlas` provide generated alpha atlases and UV rects for packed GPU
icon vertices. Hosts upload the atlas as alpha texture data and should consume
packed icon vertices when their renderer advertises support.

Generated asset files are:

- `src/tabler_svg_atlas_generated.rs` from
  `tools/rasterize_tabler_svg_atlas.py`;
- `src/lucide_svg_atlas_generated.rs` from `tools/export_lucide_svgs.mjs` and
  `tools/rasterize_lucide_svg_atlas.py`.

Do not hand-edit generated files as routine maintenance. Update the generator
or source mapping, regenerate, and run the relevant icon tests.

## Current Cleanup Queue

- Split `gpu.rs` into focused modules.
- Move web rendering setup out of the EdgeRun frontend HTML into a minimal reusable host.
- Keep old pixel-buffer UI paths deleted; new work must target GPU scene buffers.
- Keep one maintained native preview binary.
- Replace preview strings with real projected state.
- Add screenshot smoke tests for shell, launcher, full-screen system surfaces, and
  component gallery.
