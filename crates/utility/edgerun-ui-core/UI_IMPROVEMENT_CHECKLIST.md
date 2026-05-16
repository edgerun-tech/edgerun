# EdgeRun UI Core Improvement Checklist

Grounded in the current `edgerun-ui-core` layout as of May 15, 2026. This is
a planning checklist, not a claim that each item is broken. Items mark concrete
areas worth improving, validating, or documenting before the UI kit is treated
as stable public infrastructure.

## Status Snapshot

Updated May 16, 2026 after canonicalizing workspace surfaces and removing stale
app-registry guidance.

Checklist total: 135 done / 89 open.

Done or substantially in place:

- Rust-owned scene construction and host rendering boundary.
- First-class masonry and bento layouts.
- First-class drag/drop and reorder metadata/actions.
- First-class transition specs, runtime progress, easing, and transformed hit
  geometry.
- Frame-level runtime APIs for app, shell, and workspace event/tick batching.
- App registry and built-in app surfaces removed from UI core; shadcn remains
  the canonical component reference.
- Packed text/icon render buffers, scene stats, debug dumps, and renderer
  capability declarations.
- README and architecture docs for feature flags, scene ABI, host boundaries,
  fonts, icons, spacing, generated files, and frame/runtime ownership.

Highest-leverage remaining work:

- Harden scene ABI and host integration docs for WebGL2 and SDL/OpenGL.
- Move remaining browser/native host loops to the frame APIs everywhere.
- Add keyboard/focus/menu/select/command interactions as first-class runtime
  contracts.
- Split `gpu/node.rs` and component rendering into smaller infrastructure
  modules.
- Add performance evidence: frame-build budgets, allocation tracking, and
  renderer smoke benchmarks.

## Layout

- [ ] Add explicit min/max size contracts for every shell, workspace, and app surface.
- [ ] Replace scattered fixed pixel constants with named layout tokens.
- [x] Add layout tests for narrow, tablet, desktop, and wide desktop bounds.
- [x] Verify tile split ratios cannot collapse content below usable widths.
- [ ] Add deterministic overflow behavior for every app surface.
- [ ] Add text truncation or wrapping policy to every bounded label site.
- [ ] Ensure hit targets remain aligned after clipping and scrolling.
- [x] Add a shared layout pass trace for debugging computed bounds.
- [x] Consolidate workspace chrome sizing into a single exported contract.
- [x] Add stable row-height primitives for lists, tables, menus, and command palettes.
- [x] Define modal, toast, tooltip, and overlay stacking rules centrally.
- [x] Audit full-screen system surfaces for consistent safe-area handling.
- [x] Ensure launcher rows, tabs, and toolbar controls do not resize on state change.
- [x] Add layout fixtures for long app names, hashes, addresses, and policy ids.
- [x] Make masonry and bento grid layouts first-class `UiNode` layout primitives.
- [x] Add layout fixtures for empty, loading, error, and dense-data states.
- [x] Normalize scroll container padding and clip behavior across components.
- [ ] Separate layout measurement from painting where components still interleave them.
- [ ] Add visual bounds debugging overlays behind a developer feature.
- [x] Document the tiled workspace model with examples of allowed app placement.
- [x] Add regression tests for app open, close, focus, split, and tab layout.

## Rendering

- [ ] Move remaining scalar scene export compatibility toward packed buffer APIs.
- [ ] Add renderer parity tests between software, SDL/OpenGL, and WebGL paths.
- [ ] Add screenshot smoke tests for shell, launcher, workspace, lock, and gallery.
- [x] Validate clipping behavior for rects, text, icons, shadows, and hit targets.
- [x] Add explicit z-order conventions for rects, text, icons, overlays, and hits.
- [ ] Reduce per-shape draw calls in the OpenGL backend by batching compatible geometry.
- [x] Track scene statistics per frame for rects, text quads, icon quads, hits,
  drag/drop metadata, and transitions.
- [x] Add renderer error reporting for missing shader uniforms and texture setup failures.
- [ ] Ensure rounded-border and shadow shaders match visual primitives across hosts.
- [ ] Add high-DPI scaling tests for text, icons, borders, and hit projection.
- [x] Validate alpha blending order for text over translucent panels.
- [ ] Add GPU resource lifecycle tests or debug assertions for textures, buffers, and programs.
- [ ] Consolidate shader source ownership between native GL and WebGL2 modules.
- [x] Add a renderer capability struct so hosts declare supported scene features.
- [x] Add fixture scenes for pathological geometry: zero size, negative size, huge radius.
- [x] Make scene clearing semantics explicit for rects, hits, clips, text, and icons.
- [x] Add debug names or labels for scene batches to simplify renderer inspection.
- [ ] Verify icon atlas, font atlas, and explicit no-atlas geometry can coexist predictably.
- [x] Add performance budgets for frame build and frame render separately.
- [x] Document the scene ABI with versioning expectations.

## Interaction Runtime

- [x] Add first-class drag source metadata to `GpuScene`.
- [x] Add first-class drop target metadata to `GpuScene`.
- [x] Clip and hit-test drag/drop metadata through the same scene path as hits.
- [x] Add `UiNode` builders for `draggable`, `drop_target`, and `reorderable`.
- [x] Add runtime drag threshold handling.
- [x] Add runtime reorder actions with scope, item, source index, and target index.
- [x] Add runtime drag cancel behavior for Escape and blur.
- [x] Expose active drag session state for overlays/previews.
- [x] Add first-class transition specs for opacity and movement.
- [x] Add easing and deterministic transition progress in `UiRuntimeState`.
- [x] Apply transitions to emitted rects, text, icons, hits, drag sources, and drop targets.
- [x] Add frame-level `UiFrameInput` and `UiFrameOutput`.
- [x] Add frame-level shell output and workspace output APIs.
- [x] Document that hosts forward events/ticks and do not own drag/drop/transition policy.
- [ ] Migrate WebGL host loops to frame APIs instead of per-event glue.
- [x] Migrate SDL shell preview loop to frame APIs instead of per-event glue.
- [ ] Migrate remaining standalone SDL/native preview loops to frame APIs instead of per-event glue.
- [ ] Add first-class keyboard navigation contracts beyond current focus cycling.
- [ ] Add first-class menu, select, popover, tooltip, and command palette open/close contracts.
- [ ] Add first-class split/resize gestures for workspace tiles.
- [x] Add deterministic interaction replay fixtures for event sequences and frame ticks.

## Font Rendering

- [ ] Support more than one font size without rebuilding global assumptions.
- [ ] Add glyph cache behavior for runtime text outside the preloaded UI range.
- [ ] Add kerning or document why current advance-only layout is acceptable.
- [ ] Add baseline and line-height tests for common UI sizes.
- [ ] Add text clipping tests for wrapped and unwrapped labels.
- [x] Add ellipsis support for single-line bounded text.
- [ ] Add deterministic missing-glyph diagnostics for unsupported scripts.
- [x] Add fixtures for hashes, addresses, currency, policy ids, and route ids.
- [ ] Validate Inter TTC face selection and document the chosen face.
- [x] Add atlas overflow reporting instead of silently stopping glyph insertion.
- [ ] Add optional atlas sizing policy for small WASM and richer native builds.
- [x] Avoid allocating a new `Vec<TextQuad>` for every text layout where callers can stream.
- [ ] Add shaped-text extension points for future complex-script support.
- [ ] Add text measurement cache for repeated labels in shell and component lists.
- [ ] Validate pixel snapping at fractional UI scale factors.
- [x] Add tests for long unbroken tokens in wrapped text.
- [ ] Add hover/focus state text contrast checks for generated scenes.
- [x] Document font feature flags and host responsibilities for uploading atlas textures.
- [ ] Add a debug view for missing glyphs and atlas coverage.
- [x] Define when bitmap fallback text is allowed versus font atlas text.

## Icons

- [ ] Expand canonical `UiIcon` coverage for all shell and domain actions.
- [x] Add compile-time or test-time validation that every canonical icon maps to atlas data.
- [x] Add a missing-icon diagnostic path instead of quiet fallback ambiguity.
- [x] Document provider mapping differences between Tabler and Lucide names.
- [x] Generate a compact icon coverage report from the atlas import tools.
- [x] Add visual tests for common icon sizes: 12, 16, 20, 24, 32, and 40 px.
- [ ] Normalize stroke thickness between no-atlas geometry and atlas icons.
- [x] Add disabled, destructive, warning, success, and selected icon color tokens.
- [x] Remove the legacy software icon enum so `UiIcon` stays canonical.
- [x] Ensure generated atlas files include the generator command and source revision.
- [x] Add icon alignment helpers for icon-only buttons, row icons, and status badges.
- [x] Add hit target wrappers so icon-only controls remain accessible.
- [x] Add icon-only tooltip conventions for unfamiliar actions.
- [x] Validate atlas alpha format and texture upload assumptions across GL/WebGL.
- [ ] Reduce generated icon metadata footprint if only a small canonical set is used.
- [x] Add a small host-facing icon API example to the docs.
- [x] Remove app-registry icon tests with the deleted app registry.
- [ ] Add explicit no-atlas geometry for every canonical action/status icon.
- [ ] Ensure brand/product icons are not confused with status/action icons.
- [ ] Add a gallery page that renders canonical icons with names and provider ids.

## Performance

- [ ] Add frame-build microbenchmarks for shell, workspace, launcher, and gallery scenes.
- [ ] Add renderer microbenchmarks for rect-heavy, text-heavy, and icon-heavy scenes.
- [ ] Track allocations during scene build for common app surfaces.
- [ ] Reuse scene buffers instead of reallocating where host loops rebuild every frame.
- [ ] Batch rect rendering by mode, radius, shadow, and color where practical.
- [ ] Avoid repeated text measurement for static labels in components.
- [ ] Avoid string cloning in workspace surface metadata paths that can borrow static data.
- [x] Add frame-level redraw signals so unchanged UI does not repaint unnecessarily.
- [ ] Add dirty-region or dirty-scene diffing so hosts can repaint partial output.
- [ ] Add scroll virtualization for long lists, tables, audit rows, and command results.
- [ ] Add budget tests for generated atlas size and WASM binary size.
- [ ] Add feature-level size reports for `fontdue-text`, `tabler-svg-atlas`, and `lucide-svg-atlas`.
- [ ] Profile `UiNode` rendering for deep component trees.
- [ ] Reduce dynamic dispatch or match-heavy hot paths only where measurement proves value.
- [ ] Precompute stable icon atlas UVs for canonical icons instead of searching each time.
- [x] Add packed vertex buffers for text and icons where renderers still rebuild arrays.
- [ ] Add scene diffing hooks for host render loops.
- [ ] Measure cost of shadows and rounded borders on low-end GPUs.
- [ ] Add a no-font/no-atlas minimal build size target.
- [x] Add CI-friendly performance smoke checks with loose regression thresholds.
- [x] Document expected frame budgets for browser and native hosts.

## Developer Experience

- [x] Add a top-level README section for `edgerun-ui-core` build features and preview commands.
- [x] Keep one primary native preview binary and document secondary previews clearly.
- [ ] Add a single `cargo xtask`-style entrypoint for previews, icon generation, and checks.
- [ ] Add examples for building a new app surface with `UiSurfaceApp`.
- [ ] Add examples for adding a new reusable component.
- [ ] Add examples for adding a new canonical icon.
- [x] Add a scene-debug dump format that developers can inspect without a GPU host.
- [x] Add clear errors when requested fonts or icon atlas features are unavailable.
- [ ] Add rustdoc examples for `GpuScene`, `UiPainter`, `UiWorkspace`, and components.
- [x] Add contributor notes for generated files and when not to hand-edit them.
- [ ] Add a small glossary for shell, workspace, app surface, scene, hit, and host.
- [ ] Add component gallery filters by control type, domain type, and state.
- [ ] Add source links from gallery items to the Rust renderer/component.
- [x] Add environment documentation for SDL/OpenGL preview prerequisites.
- [ ] Add a common checklist for visual changes: tests, screenshots, feature flags, docs.
- [ ] Add assertions that explain invalid layout inputs with actionable messages.
- [x] Add feature matrix docs for native, WASM, SDL, GL, and no-std consumers.
- [ ] Add a minimal host integration guide for browser and native shells.
- [x] Add frame-level host boundary docs for event/tick batching.
- [x] Add common commands to the crate docs and root workspace docs.
- [x] Document the single canonical GPU scene-buffer boundary and remove legacy pixel-buffer migration ambiguity.

## Maintainability

- [ ] Continue splitting large modules, especially `gpu/node.rs`, into focused units.
- [ ] Separate component state models from render functions where they are still intertwined.
- [x] Consolidate duplicate primitive concepts by deleting the legacy pixel UI surface.
- [x] Establish ownership boundaries for shell, workspace surfaces, components, and host glue.
- [x] Add module-level invariants for hit ids and workspace surface ids.
- [ ] Replace magic hit ids with typed constants or generated ids.
- [x] Delete app-registry uniqueness guards with the removed registry.
- [ ] Reduce hardcoded preview/demo data in production-facing modules.
- [x] Move demo-only shadcn catalog surfaces behind clearly named modules/features if needed.
- [ ] Add a stable style token layer before adding more component-specific constants.
- [x] Keep compatibility shims only where active callers still need them.
- [x] Remove demo callers for the deleted legacy pixel-buffer paths.
- [x] Make generated file boundaries explicit in docs and module comments.
- [ ] Add smaller internal helpers for repeated row/card/header painting patterns.
- [ ] Keep public exports narrow and avoid re-exporting demo internals as stable API.
- [ ] Add lint expectations for unsafe GL sections and document safety requirements.
- [x] Add ownership notes for no-std versus std-only UI code.
- [x] Delete old `Painter` and pixel-buffer APIs instead of keeping a deprecation path before public release.
- [x] Document what belongs in host glue versus Rust scene construction.
- [ ] Add dependency policy notes specific to UI assets, fonts, and rendering crates.

## Spacing

- [x] Define a spacing scale and replace one-off gaps such as 6, 12, 14, and 16 px.
- [x] Add component padding tokens for dense, default, and spacious modes.
- [x] Normalize card radius and padding with the product design rules.
- [x] Normalize row icon size, row icon gap, and row text inset across components.
- [x] Add button height tokens for compact, default, and large controls.
- [x] Add toolbar and icon-button square size tokens.
- [x] Add shell/sidebar/workspace gap tokens distinct from component internal spacing.
- [x] Add tests that verify fixed controls do not resize when labels change.
- [x] Add spacing examples to the component gallery.
- [ ] Audit nested cards and replace section-card patterns with unframed layouts where needed.
- [x] Define table cell padding separately from list row padding.
- [x] Define form label, helper text, and field spacing contracts.
- [x] Normalize modal, toast, tooltip, and popover insets.
- [x] Add responsive spacing rules for narrow viewports without viewport-scaled fonts.
- [x] Add long-text fixtures to prove spacing prevents overlap.
- [x] Align app cards, package cards, proof rows, and receipt rows to shared rhythm.
- [x] Add density controls to preview compact operational UIs.
- [x] Define minimum touch/click target spacing.
- [x] Document spacing tokens in one file and one doc section.
- [ ] Add a spacing audit task to the visual change checklist.

## Components

- [x] Add state matrices for every component: default, hover, focus, active, disabled, loading, error.
- [x] Add semantic component tests that assert emitted hits and scene primitives.
- [x] Add visual examples for dense operational data, not only polished demos.
- [x] Add domain components for node instance, admission policy, and route budget surfaces.
- [x] Add first-run network app prompt as a reusable system component.
- [x] Add Trust Manager action components for open Identity, open App Store, revoke, and remove cache.
- [x] Add App Store card/detail components showing app policy hash and access mode.
- [x] Add runtime event and package proof components shared by App Store and Trust Manager.
- [x] Add import/sync source components for Gmail, Drive, GitHub, folders, and mailboxes.
- [x] Add publish-from-node components for site, app, API, and file flows.
- [x] Add data-table sorting/filtering affordances with keyboard-accessible hits.
- [x] Add menu, select, command palette, and tree view interaction tests.
- [x] Add component composition rules to avoid cards inside cards.
- [x] Add component APIs that accept projected state rather than owning demo strings.
- [x] Add error and empty states for storage, proof, admission, and app surfaces.
- [x] Add accessibility metadata coverage for each component type.
- [x] Add icon-only button and segmented-control components.
- [x] Add receipt/payment rows that distinguish payable, pending, challenged, and settled.
- [x] Add capability grant rows with policy hash, scope, expiry, and revoke affordance.
- [x] Add stable component ids or test selectors for preview and host automation.

## Documentation

- [x] Update `UI_ARCHITECTURE.md` to reflect the current module split and public exports.
- [x] Add scene ABI versioning and packed-buffer migration details.
- [x] Add module maps for shell, workspace surfaces, runtime, components, text, and icons.
- [ ] Add host integration docs for WebGL2 and SDL/OpenGL.
- [x] Add font rendering docs covering Inter assets, atlas generation, fallback, and limits.
- [x] Add icon generation docs covering Tabler imports, SVG atlas generation, and validation.
- [x] Add no-std/std/feature docs with examples for each build profile.
- [x] Remove contributor docs for adding app registry entries.
- [ ] Add contributor docs for adding app surfaces without duplicating host metadata.
- [ ] Add product terminology docs for run/cache/proof/admission/publisher language.
- [ ] Add screenshot testing docs once smoke tests exist.
- [ ] Add performance profiling docs and expected budgets.
- [ ] Add accessibility model docs and current limitations.
- [ ] Add component API docs with examples and state tables.
- [ ] Add spacing/style token docs with rationale.
- [ ] Add rendering backend docs with safety notes for unsafe GL code.
- [x] Add generated-file policy docs listing commands and source inputs.
- [ ] Add roadmap status that distinguishes done, preview-only, and production-ready.
- [x] Document that legacy `Painter`/pixel-buffer APIs are removed before public release.
- [x] Add a short "what not to put in hosts" guide for JS/SDL/native integrators.
