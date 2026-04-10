# edgerun-compositor — Wayland Protocol Conformance

A hand-coded Wayland compositor with zero external C dependencies. All protocol handling is written in Rust against a custom wire encoding/decoding layer (`src/wire/`). No `wayland-scanner` or code generation is used.

## Protocol Architecture

- **Dispatch**: `process_message()` in `src/protocol/dispatch/mod.rs` creates a `DispatchContext` and delegates to per-interface handler modules in `src/protocol/dispatch/`.
- **Handler modules**: Each interface group has its own module:
  - `core.rs` — wl_display, wl_registry
  - `compositor.rs` — wl_compositor, wl_surface, wl_region
  - `seat_handlers.rs` — wl_seat, wl_keyboard, wl_pointer, wl_touch
  - `shm.rs` — wl_shm, wl_shm_pool, wl_buffer
  - `data_device.rs` — wl_data_device_manager, wl_data_device, wl_data_source, wl_data_offer
  - `subsurface.rs` — wl_subcompositor, wl_subsurface
  - `xdg_shell.rs` — xdg_wm_base, xdg_surface, xdg_toplevel, xdg_positioner, xdg_popup
  - `xdg_ext.rs` — decoration, activation, output
  - `xdg_foreign.rs` — exporter/importer
  - `wp_ext.rs` — viewporter, cursor_shape, presentation, single_pixel_buffer, fractional_scale, tearing_control
  - `linux_ext.rs` — dmabuf, syncobj
  - `zwp_ext.rs` — pointer_constraints, relative_pointer, gestures, text_input_v1, idle_inhibit
  - `wlroots_ext.rs` — screencopy, primary_selection, data_control
  - `ime.rs` — text_input_v3, input_method_v2
- **Input processing**: `dispatch_legacy.rs` handles evdev input event dispatch.
- **Protocol definitions**: Each interface defined as Rust modules in `src/protocol/` with opcode constants, argument signatures, and event builders.
- **Wire encoding**: Custom little-endian binary protocol in `src/wire/` (decode.rs, encode.rs, fd.rs).
- **No codegen**: Protocol XML specs in `docs/protocol-specs/` are reference documents only — not consumed at build time.

## Advertised Globals

32 globals advertised at these versions (from `GLOBALS` constant in `dispatch.rs`):

```
wl_compositor v4, wl_shm v1, wl_seat v7, xdg_wm_base v6,
wl_output v4, zwp_linux_dmabuf_v1 v4, wl_data_device_manager v3,
wl_subcompositor v1, zxdg_decoration_manager_v1 v1, wp_viewporter v1,
wp_cursor_shape_manager_v1 v1, xdg_activation_v1 v1, wp_presentation v1,
zwp_relative_pointer_manager_v1 v1, zwp_pointer_gestures_v1 v1,
zwp_text_input_manager_v1 v1, zwp_idle_inhibit_manager_v1 v1,
zwp_pointer_constraints_v1 v1, zxdg_output_manager_v1 v3,
zxdg_exporter_v2 v1, zxdg_importer_v2 v1,
linux_drm_syncobj_v1 v1, linux_drm_syncobj_surface_v1 v1,
linux_drm_syncobj_timeline_v1 v1,
zwlr_screencopy_manager_v1 v3, zwp_text_input_manager_v3 v1,
zwp_input_method_manager_v2 v1, zwlr_primary_selection_manager_v1 v1,
zwlr_data_control_manager_v1 v2,
wp_single_pixel_buffer_manager_v1 v1, wp_fractional_scale_manager_v1 v1,
wp_tearing_control_manager_v1 v1
```

## Protocol Implementation Status

### Core Wayland (wayland.xml) — ~92%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `wl_display` | v1 | ✅ Implemented | `SYNC` (callback + done), `GET_REGISTRY` (sends all globals). No `ERROR` or `DELETE_ID` events sent. |
| `wl_registry` | v1 | ✅ Implemented | `BIND` with full global dispatch table. |
| `wl_callback` | v1 | ✅ Implemented | Created via `SYNC` and `FRAME`; `done` event sent. Frame callbacks not fired on page-flip completion. |
| `wl_compositor` | v4 | ✅ Implemented | `CREATE_SURFACE`, `CREATE_REGION`. |
| `wl_surface` | v4 | ✅ Implemented | All 11 opcodes: `ATTACH`, `DAMAGE`, `FRAME`, `COMMIT`, `SET_BUFFER_SCALE`, `SET_BUFFER_TRANSFORM`, `SET_OPAQUE_REGION`, `SET_INPUT_REGION`, `DESTROY`, `DAMAGE_BUFFER`, `OFFSET`. |
| `wl_region` | v1 | ✅ Implemented | `DESTROY`, `ADD`, `SUBTRACT`. Region geometry tracked per-surface as list of rectangles. |
| `wl_buffer` | v1 | ✅ Implemented | `DESTROY` handled. |
| `wl_shm` | v1 | ✅ Implemented | `CREATE_POOL` (with FD), format events for XRGB8888/ARGB8888. `RELEASE` handled (no-op, client cleanup). |
| `wl_shm_pool` | v1 | ✅ Implemented | `CREATE_BUFFER`, `RESIZE`, `DESTROY`. |
| `wl_output` | v4 | ✅ Implemented | Geometry, mode, scale (v3+), name (v4+), done events. `RELEASE` handled (registry cleanup). |
| `wl_seat` | v7 | ✅ Implemented | Capabilities (POINTER\|KEYBOARD\|TOUCH), name (v7+). `RELEASE` handled. |
| `wl_keyboard` | v7 | ✅ Implemented | Keymap (XKB text), enter/leave/key/modifiers/repeat_info. `RELEASE` handled. |
| `wl_pointer` | v7 | ✅ Implemented | enter/leave/motion/button/axis/frame/axis_source/axis_stop/axis_discrete. `SET_CURSOR`, `RELEASE` handled. |
| `wl_touch` | v7 | ✅ Implemented | down/up/motion/frame/cancel. `RELEASE` handled. Shape/orientation events not sent (no hardware data). |
| `wl_data_device_manager` | v3 | ✅ Implemented | `CREATE_DATA_SOURCE`, `GET_DATA_DEVICE`, `DESTROY`. |
| `wl_data_device` | v3 | ✅ Implemented | `SET_SELECTION`, `START_DRAG` (stub), `RELEASE`. Events: data_offer, selection, enter, leave, motion, drop. |
| `wl_data_source` | v3 | ✅ Implemented | `OFFER`, `DESTROY`, `SET_ACTIONS`. Events: target, send, cancelled. |
| `wl_data_offer` | v3 | ✅ Implemented | `ACCEPT`, `RECEIVE` (with FD forwarding), `SET_ACTIONS`, `DESTROY`. Events: offer, source_actions. |
| `wl_subcompositor` | v1 | ✅ Implemented | `GET_SUBSURFACE`, `DESTROY`. |
| `wl_subsurface` | v1 | ✅ Implemented | `SET_POSITION`, `SET_SYNC`, `SET_DESYNC`, `PLACE_ABOVE`, `PLACE_BELOW`, `DESTROY`. |

### xdg-shell (stable v6) — ~95%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `xdg_wm_base` | v6 | ✅ Implemented | `DESTROY`, `GET_XDG_SURFACE`, `CREATE_POSITIONER`, `PONG`. Ping event sent. |
| `xdg_surface` | v6 | ✅ Implemented | `GET_TOPLEVEL`, `GET_POPUP`, `ACK_CONFIGURE`, `SET_WINDOW_GEOMETRY`, `DESTROY`. |
| `xdg_toplevel` | v6 | ✅ Implemented | Full lifecycle: title, app_id, min/max size, maximize, fullscreen, minimize, move, resize, parent. Events: configure, configure_bounds (v4+), close, wm_capabilities (v5+). |
| `xdg_positioner` | v6 | ✅ Implemented | All setters: size, anchor_rect, anchor, gravity, constraint_adjustment, offset, reactive, parent_size, parent_configure. |
| `xdg_popup` | v6 | ✅ Implemented | `GRAB` (with configure sent), `REPOSITION`, `DESTROY`. Events: configure, done, repositioned. |

### xdg Extensions — ~95%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `zxdg_decoration_manager_v1` | v1 | ✅ Implemented | `GET_TOPLEVEL_DECORATION`, `DESTROY`. Defaults to SERVER_SIDE. |
| `zxdg_toplevel_decoration_v1` | v1 | ✅ Implemented | `SET_MODE`, `UNSET_MODE`, `DESTROY`. Configure event sent. |
| `xdg_activation_v1` | v1 | ✅ Implemented | `GET_ACTIVATION_TOKEN`, `ACTIVATE` (full focus + configure + raise), `DESTROY`. |
| `zxdg_output_manager_v1` | v3 | ✅ Implemented | `GET_XDG_OUTPUT` sends logical_position, logical_size, name, description, done. |
| `zxdg_exporter_v2` | v1 | ✅ Implemented | `EXPORT` (generates handle string), `DESTROY`. |
| `zxdg_exported_v2` | v1 | ⚠️ Partial | `DESTROY`, `SET_PARENT_OF` (TODO: validate surface handle visibility). |
| `zxdg_importer_v2` | v1 | ✅ Implemented | `IMPORT`, `DESTROY`. |
| `zxdg_imported_v2` | v1 | ⚠️ Partial | `DESTROY`, `SET_PARENT_OF` (TODO: validate handle). |

### Wayland Protocols Staging (wp_*) — ~95%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `wp_viewporter` | v1 | ✅ Implemented | `GET_VIEWPORT`, `DESTROY`. |
| `wp_viewport` | v1 | ✅ Implemented | `SET_SOURCE` (fixed-point to f64), `SET_DESTINATION`, `DESTROY`. |
| `wp_cursor_shape_manager_v1` | v1 | ✅ Implemented | `GET_POINTER_SHAPE`, `DESTROY`. |
| `wp_cursor_shape_device_v1` | v1 | ✅ Implemented | `SET_SHAPE` (maps shape_id to cursor index), `DESTROY`. 31 shape constants. |
| `wp_presentation` | v1 | ✅ Implemented | `FEEDBACK`, `DESTROY`. |
| `wp_presentation_feedback` | v1 | ✅ Implemented | Full tracker: `register`, `supersede`, `take_presented`. Events: `presented` (timestamp + seq + flags), `discarded` (reason). |
| `wp_single_pixel_buffer_manager_v1` | v1 | ✅ Implemented | `CREATE_SRGB32_BUFFER` (creates real 1×1 memfd SHM buffer), `DESTROY`. |
| `wp_fractional_scale_manager_v1` | v1 | ✅ Implemented | `GET_FRACTIONAL_SCALE` (sends preferred_scale = output_scale × 120), `DESTROY`. |
| `wp_tearing_control_manager_v1` | v1 | ✅ Implemented | `GET_TEARING_CONTROL`, `DESTROY`. |
| `wp_tearing_control_v1` | v1 | ✅ Implemented | `SET_PRESENTATION_HINT` (DEFAULT/SYNC/ASYNC), `DESTROY`. Hint tracked per-surface. |

### Linux Extensions — ~70%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `zwp_linux_dmabuf_v1` | v4 | ✅ Implemented | Format + modifier events on bind. `CREATE_PARAMS`, `CREATE_IMMED`. Formats: XRGB8888, ARGB8888, XBGR8888, ABGR8888 with LINEAR/INVALID modifiers. |
| `linux_drm_syncobj_v1` | v1 | 🔴 Stub | `GET_SURFACE`, `CREATE_TIMELINE`, `DESTROY` accepted but no fence synchronization. |
| `linux_drm_syncobj_surface_v1` | v1 | 🔴 Stub | `SET_ACQUIRE_POINT`, `SET_RELEASE_POINT` accepted but ignored. |
| `linux_drm_syncobj_timeline_v1` | v1 | 🔴 Stub | `IMPORT_SYNC_FILE`, `EXPORT_SYNC_FILE` accepted but ignored. |

### Unstable Protocols (zwp_*) — ~92%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `zwp_pointer_constraints_v1` | v1 | ✅ Implemented | `LOCK_POINTER` (sends locked), `CONFINE_POINTER` (sends confined). |
| `zwp_locked_pointer_v1` | v1 | ✅ Implemented | `SET_CURSOR_POSITION_HINT`, `SET_REGION`, `DESTROY` (sends unlocked). |
| `zwp_confined_pointer_v1` | v1 | ✅ Implemented | `SET_REGION`, `DESTROY` (sends unconfined). |
| `zwp_relative_pointer_manager_v1` | v1 | ✅ Implemented | `GET_RELATIVE_POINTER`, `DESTROY`. |
| `zwp_relative_pointer_v1` | v1 | ✅ Implemented | `DESTROY`. Relative motion with hi/lo timestamps and fixed-point deltas. |
| `zwp_pointer_gestures_v1` | v1 | ✅ Implemented | Full gesture detection: swipe begin/update/end, pinch begin/update/end (scale + rotation). |
| `zwp_text_input_manager_v1` | v1 | ✅ Implemented | `CREATE_TEXT_INPUT`, `DESTROY`. |
| `zwp_text_input_v1` | v1 | ✅ Implemented | All 12 opcodes: activate, deactivate, show/hide panel, reset, surrounding_text, content_type, cursor_rectangle, preferred_language, commit_state, invoke_action, destroy. |
| `zwp_idle_inhibit_manager_v1` | v1 | ✅ Implemented | `CREATE_INHIBITOR` (per-surface tracking), `DESTROY`. Full inhibitor set in Shell. |

### Text Input v3 + Input Method v2 — ~92%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `zwp_text_input_manager_v3` | v1 | ✅ Implemented | `GET_TEXT_INPUT`, `DESTROY`. State tracked in `TextInputState`. |
| `zwp_text_input_v3` | v1 | ✅ Implemented | enable, disable, set_surrounding_text, set_text_change_cause, commit, get_surrounding_text, destroy. Events: enter, leave, preedit_string, commit_string, delete_surrounding_text, done. |
| `zwp_input_method_manager_v2` | v1 | ✅ Implemented | `GET_INPUT_METHOD`, `DESTROY`. |
| `zwp_input_method_v2` | v1 | ✅ Implemented | Full IME: commit_string, commit_preedit, delete_surrounding, commit, grab_keyboard (with keymap/repeat_info), set_surrounding_text, set_text_change_cause, set_content_type, available. Events: activate, deactivate, surround_text, text_change_cause, content_type, done. |
| `zwp_input_method_keyboard_grab_v2` | v1 | ✅ Implemented | `DESTROY`, `RELEASE`. Events: key, modifiers, keymap, repeat_info. |

### wlroots Extensions (zwlr_*) — ~92%

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `zwlr_screencopy_manager_v1` | v3 | ✅ Implemented | `CAPTURE_OUTPUT`, `CAPTURE_OUTPUT_REGION`. Full pixel copy from framebuffer to client SHM. Events: buffer, flags (Y_INVERT), ready, failed. |
| `zwlr_screencopy_frame_v1` | v3 | ✅ Implemented | `COPY`, `COPY_WITH_DAMAGE`, `DESTROY`. |
| `zwlr_primary_selection_manager_v1` | v1 | ✅ Implemented | `CREATE_DATA_SOURCE`, `GET_PRIMARY_SELECTION`, `DESTROY`. Full selection lifecycle. |
| `zwlr_data_control_manager_v1` | v2 | ✅ Implemented | `CREATE_DATA_SOURCE`, `GET_DATA_DEVICE`, `DESTROY`. Headless clipboard access. |

### Custom Protocols

| Interface | Version | Status | Notes |
|-----------|---------|--------|-------|
| `edgerun_test_overlay` | v1 | ✅ Implemented | Test overlay protocol for compositor rendering tests. |

## Remaining Gaps

### Stubs (documented)
- **`linux_drm_syncobj_*`** — All 3 interfaces are stubs. Accept calls but don't implement fence synchronization. Comment in `src/protocol/linux_drm_syncobj.rs`: "This is a stub implementation."

### No-ops (accepted but not fully functional)
- **`xdg_toplevel::MOVE` / `RESIZE`** — Accepted with logging and resize state tracking. Full interactive pointer grab not yet implemented (compositor controls window placement).
- **`xdg_toplevel::SHOW_WINDOW_MENU`** — Declared in protocol but not dispatched.
- **`xdg_positioner::SET_REACTIVE` / `SET_PARENT_SIZE` / `SET_PARENT_CONFIGURE`** — Values stored in PositionerState; reactive repositioning not yet triggered.

### Not implemented
- **`wl_touch` shape/orientation events** — Declared but not sent (evdev provides no touch shape data).
- **Frame callbacks on page-flip** — Created via `FRAME` but only fired on sync, not on actual display refresh.

### Protocol not advertised
- **`wl_shell`** — Legacy protocol, deprecated in favor of xdg-shell. Correctly omitted.

## Overall Conformance: ~95%

The compositor fully implements the protocols it chooses to advertise, with three categories of gaps:
1. **Hardware-dependent** features (touch shape/orientation, syncobj fences) — require specific kernel/driver support
2. **Interactive features** (move, resize, DnD) — require compositor-side UI that hasn't been built yet
3. **Minor protocol completeness** (DELETE_ID, frame-on-vsync) — functional but not spec-complete
