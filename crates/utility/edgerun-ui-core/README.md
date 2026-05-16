# edgerun-ui-core

`edgerun-ui-core` is the shared EdgeRun Rust UI kit. Rust builds `GpuScene`
command buffers; browser, SDL, and native hosts render those buffers and
forward input back into Rust.

## Build Profiles

| Consumer | Command | Notes |
| --- | --- | --- |
| Portable core | `cargo test -p edgerun-ui-core --no-default-features` | Smallest build; no font atlas, no generated icon atlas, no `std` UI host helpers. |
| Default UI | `cargo test -p edgerun-ui-core` | Enables `fontdue-text`, `tabler-svg-atlas`, and `lucide-svg-atlas`. |
| Host APIs | `cargo check -p edgerun-ui-core --features std` | Enables `std` without requiring font rasterization. |
| Font atlas path | `cargo check -p edgerun-ui-core --features fontdue-text` | Enables Inter atlas layout and text quad emission. |
| UI showcase preview | `cargo run -p edgerun-ui-core --features sdl --bin ui-preview-sdl-shadcn` | Runs the canonical shadcn component showcase through SDL/OpenGL. |

The default feature set is asset-rich for UI development. Consumers that need
the smallest portable core must pass `--no-default-features` explicitly.

## Preview Binaries

The primary native preview is `ui-preview-sdl-shadcn`. It opens the UI Core
canonical shadcn showcase. It is the visual reference for reusable components,
layout, typography, icons, and interaction behavior. Use this preview when
validating public UI work.

SDL/OpenGL previews require SDL2 development libraries, an OpenGL-capable
session, and access to a display server. Headless CI should build these binaries
and use scene/debug tests; screenshot smoke tests should run only on workers
with a real or virtual GL display.

## Feature Matrix

| Feature | Enables | Host responsibility |
| --- | --- | --- |
| `std` | GPU module and host-facing helpers that require allocation and OS-facing glue. | Keep OS handles, windows, timers, and files in host glue. |
| `fontdue-text` | Inter font atlas construction, glyph metrics, text quads, and host frame helpers. | Upload the atlas as an alpha texture and report atlas overflow from `FontAtlas::atlas_overflowed()`. |
| `tabler-svg-atlas` | Generated Tabler alpha atlas and icon quad emission. | Upload the atlas as an alpha texture and consume packed icon vertices when available. |
| `lucide-svg-atlas` | Generated Lucide alpha atlas and icon quad emission. | Use only with a valid Lucide asset pack so every canonical `UiIcon` has a provider entry. |
| `gpu-gl` | Native OpenGL renderer backend. | Own GL context creation and surface lifecycle outside core UI logic. |
| `sdl` | SDL input/event adapter and preview loop helpers. | Depends on `gpu-gl` and `fontdue-text`; keep app/runtime setup outside core UI logic. |

## Scene And Hosts

Use `GpuScene::stats()` and `GpuScene::debug_dump()` when inspecting a frame
without a renderer. Hosts should prefer `PackedGpuScene::pack()` and check
`UiRendererCapabilities.scene_abi_version` before consuming packed buffers.
Packed scene buffers are the canonical host boundary.

Frame-size budgets are explicit ABI-side contracts, not renderer folklore.
Use `GpuSceneBudget::public_showcase_frame()` for demos intended to prove the
system publicly, `browser_interactive_frame()` for browser hosts, and
`native_interactive_frame()` for richer native shells. Tests should compare
`GpuScene::stats()` against the relevant budget before adding large surfaces,
deep component trees, or generated visual assets.
Use the matching `GpuFrameTimeBudget` when a host or benchmark has measured
build and render timings; build and render ceilings stay separate so renderer
work does not hide scene-construction regressions.

Host glue should not implement layout or duplicate scene/runtime behavior.
Production app metadata, app launching, capability semantics, and Trust Manager
state belong outside UI core. There is no app registry, built-in app catalog, or
production app surface model in this crate. The shadcn showcase remains in core
because it is the canonical component reference, not an app registry.

Apps should expose one `UiSurfaceApp` implementation that builds a single
`UiNode` surface for the provided viewport and handles semantic `UiAction`
values. Do not wire pointer capture, text input, scroll wheels, drag/drop,
focus, or transition ticks in app crates. Use `instantiate_sdl_app(...)` for SDL
or `instantiate_webgl_app(...)` / `UiSurfaceHost` for packed WebGL scene frames;
UI core owns input capture, runtime forwarding, scene construction, and buffer
packing. Apps may handle normalized `UiEvent` values for app-level shortcuts or
projected state, but hosts still own raw device capture and runtime hit testing.

Canonical app crates should import through `edgerun_ui_core::gpu::prelude`:

```rust
use edgerun_ui_core::gpu::prelude::*;

struct MyApp;

impl UiSurfaceApp for MyApp {
    fn surface(&mut self, _viewport: UiRect) -> UiNode {
        column("gap-4 p-4").child(text("Network app")).class("bg-background")
    }

    fn handle_action(&mut self, action: UiAction) -> UiAppControl {
        let _ = action;
        UiAppControl::clean()
    }
}
```

Lower-level direct SDL rendering is capability-gated through
`SdlDirectDrawCapability` and exists for diagnostics or trusted framework
adapters. Product app crates should not depend on it.

## Spacing And Layout

Reusable spacing lives in `src/gpu/spacing.rs`. Add shared dimensions there
when a value is used by shell, workspace, or multiple components. Keep local
one-off geometry near the component until it becomes a public layout contract.
Component padding has dense, default, and spacious tokens; row icon slots,
icon gaps, and text insets are also shared contracts so list rows, proof rows,
payments, and menus do not drift independently.

First-class node layouts include stack rows/columns, responsive grids, masonry
layouts, and bento grids. Use `masonry_auto_for_width` for variable-height card
feeds and `bento_grid_auto_for_width` for dense dashboard or product surfaces
with spanned cards.

## First-Class Interactions

Dragging and reordering are owned by Rust UI core. Mark reorderable nodes with
`UiNode::reorderable(scope_id, item_id, index)` or use the lower-level
`draggable` and `drop_target` builders. Rendering emits `GpuDragSource` and
`GpuDropTarget` metadata into `GpuScene`, and `UiRuntimeState::handle_event`
turns pointer input into `DragStarted`, `DragMoved`, `Dropped`, `Reordered`,
and `DragCancelled` actions.

Hosts should forward pointer/key events and apply returned actions to app state.
They should not reimplement drag thresholds, drop hit testing, or list/grid
reordering in JavaScript or SDL glue.

Transitions follow the same rule. Use `fade`, `slide_x`, `slide_y`, or
`transition(UiTransitionSpec)` on a node. Rendering records the transition in
`GpuScene`, and `UiRuntimeState::sync_transitions` plus
`advance_transitions(delta_ms)` owns progress and easing. Hosts provide frame
ticks and redraw when Rust reports more transition work.

For host loops, prefer `UiRuntimeState::handle_frame(scene, UiFrameInput)` over
calling lower-level event and transition methods directly. `UiFrameInput`
contains viewport metadata, scale, frame delta, and ordered events.
`UiFrameOutput` returns semantic actions plus redraw and active-transition
signals. Shell and workspace surfaces expose the same pattern through
`UiShellState::handle_frame` and `UiWorkspace::handle_frame`.

The tiled workspace model is owned by `src/gpu/workspace.rs`: surfaces render into
assigned bounds, do not float over each other, and do not own global shell
chrome.

## System Components

Reusable system components live in `src/gpu/components.rs` and render through
`UiPainter`. `network_app_prompt` owns the first-run network app choice with
the canonical actions `[Run once] [Verify & cache] [Cancel]`; callers provide
the button ids and projected package metadata, then handle the emitted
`HitKind::Button` actions. `trust_manager_actions` owns the common proof
dashboard actions for opening Identity, opening App Store, revoking grants, and
removing verified cache bytes. `app_store_card` shows the package hash, app
policy hash, access mode, and Run action for a network app without treating
catalog discovery as authority. `runtime_event_row` and `package_proof_row`
share proof and package event rendering between App Store and Trust Manager
surfaces, including stable `ListRow` hits for host automation and action
routing. `import_sync_source_row` covers Gmail, Drive, GitHub, folder, and
mailbox import/sync sources as projected state rows with policy hash display,
status, configure action, and sync action ids. `publish_from_node_row` covers
site, app, API, and file publishing from a node instance with route scope,
publisher policy hash, budget, status, configure action, and publish action
ids. `node_instance_row`, `admission_policy_row`, and `route_budget_row`
expose the node role instance, admission policy commitment, and admitted route
budget surfaces as shared projected-state rows. `data_table_controls` provides
shared filter and sortable column header affordances using `Input` and `Button`
hits so hosts can route pointer and keyboard activation without table-specific
JavaScript or SDL behavior. Menu items, selects, command palettes, and tree
items are covered by runtime keyboard interaction tests so hosts can rely on
the emitted hit kinds for focus, activation, opening, and text input.
`receipt_payment_row` uses typed payment states for payable, pending,
challenged, and settled receipts, with deterministic status labels, colors,
transaction row hits, and optional action buttons. `capability_grant_detail_row`
exposes app, capability, scope, policy hash, expiry, status, and a revoke
action as a shared Trust Manager row. `system_surface_state_panel` gives
storage, proof, admission, and app surfaces deterministic empty/error states
with references and optional action hits. `UiComponentTestId` and
`UI_COMPONENT_TEST_IDS` provide stable namespaced selectors for preview and
host automation without requiring hosts to infer component identity from
display text. `UiComponentStateMatrix` and `UI_COMPONENT_STATE_MATRICES`
document default, hover, focus, active, disabled, loading, and error coverage
for every stable component selector. `UiNode::composition_issues` reports
invalid component composition such as nested cards; use unframed rows, columns,
grids, or sections inside a card instead of placing another card inside it.
`UiComponentProjectionContract` and
`UI_COMPONENT_PROJECTION_CONTRACTS` list the projected fields each stable
component consumes so callers provide runtime/app state explicitly instead of
letting reusable components own demo data. `UiComponentAccessibilityMetadata`
and `UI_COMPONENT_ACCESSIBILITY_METADATA` bind every stable component selector
to an accessibility role and projected label fields. `icon_only_button` and
`segmented_control` provide reusable tool-button and segmented-mode controls
with fixed hit geometry and projected action ids. The shadcn showcase is the
canonical component reference for visual and interaction quality.

## Font Assets

Inter font bytes live at `assets/Inter.ttc`; Geist variable bytes live at
`assets/Geist-Variable.ttf`; GPU text support lives in `src/gpu/text.rs`. With
`fontdue-text`, `FontAtlas::load_inter(px)` or `FontAtlas::load_geist(px)`
builds an alpha atlas for the requested UI size. Geist support is variable-face
only; do not add static Geist loaders or alternate asset paths. Unsupported
glyphs and atlas overflow are observable through `skipped_glyph_count()` and
`atlas_overflowed()` so missing coverage cannot pass silently.

Hosts upload font atlas alpha bytes and render text quads from the scene or
packed text vertex buffer. Bitmap text is the explicit no-font build path for
debug/minimal hosts, not the production UI text path.

## Icon Assets

New UI code should use canonical `UiIcon` values. Generated SVG alpha atlases
back those icons when the corresponding features are enabled; tests validate
that canonical icons resolve to provider names and atlas rects. Runtime
replacement is pack-validated, so a host can switch icon and font providers only
after the replacement covers the same canonical requirements.

Host-facing icon example:

```rust
use edgerun_ui_core::gpu::{
    lucide_svg_icon_atlas, Color4, GpuScene, UiIcon, UiPainter, UiRect,
    EDGERUN_LUCIDE_GEIST_ASSET_PACK,
};

let icon_atlas = lucide_svg_icon_atlas();
host_upload_alpha_texture(icon_atlas.width, icon_atlas.height, icon_atlas.alpha);

let mut scene = GpuScene::new();
let mut ui = UiPainter::new(&mut scene).with_asset_pack(EDGERUN_LUCIDE_GEIST_ASSET_PACK)?;
ui.icon(
    UiRect::new(16.0, 16.0, 20.0, 20.0),
    UiIcon::Shield,
    Color4::rgba(0.0, 0.0, 0.0, 1.0),
);
```

Generation sources and commands:

- `src/tabler_svg_atlas_generated.rs`: generated from `@tabler/icons` SVG files
  by `tools/rasterize_tabler_svg_atlas.py`.
- `src/lucide_svg_atlas_generated.rs`: generated from `@lucide/icons` SVG files
  by `tools/export_lucide_svgs.mjs` and `tools/rasterize_lucide_svg_atlas.py`.
- Icon coverage report: `tools/report_icon_coverage.py`.

Do not hand-edit generated files unless the generator cannot run and the repair
is documented in the change. Prefer updating the generator or canonical icon
mapping, then regenerating and running icon validation tests.

## Common Checks

```bash
cargo fmt --package edgerun-ui-core
cargo test -p edgerun-ui-core
cargo test -p edgerun-ui-core --no-default-features
cargo check -p edgerun-ui-core --features std
cargo check -p edgerun-ui-core --features fontdue-text
cargo check -p edgerun-ui-core --no-default-features --features std
cargo build -p edgerun-ui-core --features sdl --bin ui-preview-sdl-shadcn
```
