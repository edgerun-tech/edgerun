# Wayland Protocol Specifications

Official XML protocol specs from upstream sources. Every request, event, enum,
and interface in our compositor should match what's defined here.

## Layout

```
docs/protocol-specs/
├── core/          # Core Wayland protocols (wayland.xml)
├── xdg/           # XDG shell + extensions (freedesktop)
├── wp/            # Wayland-protocols staging extensions
├── zwp/           # Unstable protocols (zwp_ prefix)
├── zwlr/          # Wlroots extensions (zwlr_ prefix)
└── linux/         # Linux-specific extensions
```

## Protocol → Spec Mapping

| Our Protocol File | Spec File | Lines |
|-------------------|-----------|-------|
| `wl_core.rs` (wl_display, wl_registry, wl_callback) | `core/wl_core.xml` | 2892 |
| `wl_compositor.rs` (wl_compositor, wl_surface, wl_region) | `core/wl_core.xml` | 2892 |
| `wl_shm.rs` (wl_shm, wl_shm_pool, wl_buffer) | `core/wl_core.xml` | 2892 |
| `wl_output.rs` (wl_output) | `core/wl_core.xml` | 2892 |
| `wl_seat.rs` (wl_seat, wl_keyboard, wl_pointer, wl_touch) | `core/wl_core.xml` | 2892 |
| `wl_data_device.rs` (wl_data_device_manager, wl_data_device, wl_data_source, wl_data_offer) | `core/wl_core.xml` | 2892 |
| `wl_subcompositor.rs` (wl_subcompositor, wl_subsurface) | `core/wl_core.xml` | 2892 |
| `xdg_shell.rs` (xdg_wm_base, xdg_surface, xdg_toplevel, xdg_popup) | `xdg/xdg-shell.xml` | 1418 |
| `xdg_output.rs` | `xdg/xdg-output.xml` | 222 |
| `xdg_decoration.rs` | `xdg/xdg-decoration-unstable-v1.xml` | 176 |
| `xdg_foreign.rs` | `xdg/xdg-foreign-unstable-v2.xml` | 200 |
| `wp_viewporter.rs` | `wp/viewporter.xml` | 177 |
| `wp_cursor_shape.rs` | `wp/cursor-shape-v1.xml` | 162 |
| `wp_presentation_time.rs` | `wp/presentation-time.xml` | 268 |
| `linux_dmabuf.rs` | `linux/linux-dmabuf-v1.xml` | 585 |
| `linux_drm_syncobj.rs` | `linux/linux-drm-syncobj-v1.xml` | 261 |
| `zxdg_idle_inhibit.rs` | `zwp/idle-inhibit-unstable-v1.xml` | 83 |
| `zwp_pointer_constraints.rs` | `zwp/pointer-constraints-unstable-v1.xml` | 334 |
| `zwp_relative_pointer.rs` | `zwp/relative-pointer-unstable-v1.xml` | 136 |
| `zwp_pointer_gestures.rs` | `zwp/pointer-gestures-unstable-v1.xml` | 277 |
| `zwp_text_input.rs` | `zwp/text-input-unstable-v1.xml` | 385 |
| `text_input_v3.rs` | `zwp/text-input-unstable-v3.xml` | 604 |
| `screencopy.rs` | `zwlr/wlr-screencopy-unstable-v1.xml` | 231 |
| `data_control.rs` | `zwlr/wlr-data-control-unstable-v1.xml` | 278 |

## Specs Not Available Upstream

| Our Protocol File | Notes |
|-------------------|-------|
| `xdg_activation.rs` | xdg-activation-v1.xml removed from upstream — was promoted to stable under different structure |
| `input_method_v2.rs` | input-method-unstable-v2.xml directory removed from upstream — may have been superseded |
| `primary_selection.rs` | wlr-primary-selection — wlroots-specific, upstream repo structure changed |

## Source Repos

| Directory | Source |
|-----------|--------|
| `core/` | [wayland](https://gitlab.freedesktop.org/wayland/wayland) |
| `xdg/` `wp/` `zwp/` `linux/` | [wayland-protocols](https://gitlab.freedesktop.org/wayland/wayland-protocols) |
| `zwlr/` | [wlroots](https://gitlab.freedesktop.org/wlroots/wlroots) |

## Updating Specs

```bash
cd docs/protocol-specs

# Core wayland
curl -sL "https://gitlab.freedesktop.org/wayland/wayland/-/raw/master/protocol/wayland.xml" \
  -o core/wl_core.xml

# Stable protocols (from wayland-protocols)
BASE="https://gitlab.freedesktop.org/wayland/wayland-protocols/-/raw/main"
curl -sL "$BASE/stable/xdg-shell/xdg-shell.xml" -o xdg/xdg-shell.xml
curl -sL "$BASE/stable/viewporter/viewporter.xml" -o wp/viewporter.xml
curl -sL "$BASE/stable/xdg-output/xdg-output.xml" -o xdg/xdg-output.xml 2>/dev/null || \
  curl -sL "https://cgit.freedesktop.org/wayland/wayland-protocols/plain/unstable/xdg-output/xdg-output-unstable-v1.xml" -o xdg/xdg-output.xml
curl -sL "$BASE/stable/linux-dmabuf/linux-dmabuf-v1.xml" -o linux/linux-dmabuf-v1.xml
curl -sL "$BASE/stable/presentation-time/presentation-time.xml" -o wp/presentation-time.xml

# Staging
curl -sL "$BASE/staging/cursor-shape/cursor-shape-v1.xml" -o wp/cursor-shape-v1.xml
curl -sL "$BASE/staging/linux-drm-syncobj/linux-drm-syncobj-v1.xml" -o linux/linux-drm-syncobj-v1.xml

# Unstable
curl -sL "$BASE/unstable/idle-inhibit/idle-inhibit-unstable-v1.xml" -o zwp/idle-inhibit-unstable-v1.xml
curl -sL "$BASE/unstable/pointer-constraints/pointer-constraints-unstable-v1.xml" -o zwp/pointer-constraints-unstable-v1.xml
curl -sL "$BASE/unstable/relative-pointer/relative-pointer-unstable-v1.xml" -o zwp/relative-pointer-unstable-v1.xml
curl -sL "$BASE/unstable/pointer-gestures/pointer-gestures-unstable-v1.xml" -o zwp/pointer-gestures-unstable-v1.xml
curl -sL "$BASE/unstable/text-input/text-input-unstable-v1.xml" -o zwp/text-input-unstable-v1.xml
curl -sL "$BASE/unstable/text-input/text-input-unstable-v3.xml" -o zwp/text-input-unstable-v3.xml
curl -sL "$BASE/unstable/xdg-decoration/xdg-decoration-unstable-v1.xml" -o xdg/xdg-decoration-unstable-v1.xml
curl -sL "$BASE/unstable/xdg-foreign/xdg-foreign-unstable-v2.xml" -o xdg/xdg-foreign-unstable-v2.xml
curl -sL "$BASE/unstable/input-method/input-method-unstable-v2.xml" -o zwp/input-method-unstable-v2.xml

# Wlroots extensions
WLR="https://gitlab.freedesktop.org/wlroots/wlroots/-/raw/master/protocol"
curl -sL "$WLR/wlr-screencopy-unstable-v1.xml" -o zwlr/wlr-screencopy-unstable-v1.xml
curl -sL "$WLR/wlr-data-control-unstable-v1.xml" -o zwlr/wlr-data-control-unstable-v1.xml
curl -sL "$WLR/wlr-primary-selection-unstable-v1.xml" -o zwlr/wlr-primary-selection-unstable-v1.xml

# Validate all
for f in $(find . -name '*.xml'); do
  if head -1 "$f" | grep -q '<?xml'; then echo "OK   $f"; else echo "FAIL $f"; fi
done
```
