# edgerun-ui-core

Tiny software UI primitives for EdgeRun.

This crate is deliberately not a browser, DOM, CSS engine, GTK/Qt wrapper, or
Wayland toolkit. It draws a small system UI kit into a caller-owned XRGB8888 /
ARGB8888 pixel buffer.

Intended consumers:

- `edgerun-compositor` for system chrome, launcher, overlays, setup screens.
- `edgerun-term` for command palettes, cards, inline panels, and debug UI.
- browser/canvas host later, using the same semantic UI model.

The first API is immediate-mode because it is the smallest useful layer:
`Painter` owns no allocation and only mutates the supplied framebuffer slice.

```rust
use edgerun_ui_core::{demo_dashboard, DashboardState, EDGERUN_DARK, Painter};

let mut painter = Painter {
    pixels: framebuffer,
    width,
    height,
    pitch,
};

demo_dashboard(&mut painter, DashboardState::default(), EDGERUN_DARK);
```

Next steps:

1. Wire `demo_dashboard` as an optional compositor overlay.
2. Move terminal palettes/cards to this shared renderer.
3. Add retained `UiNode` + hit testing once the visual primitives are stable.
