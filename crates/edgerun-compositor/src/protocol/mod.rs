//! Protocol interface definitions.
//!
//! Each Wayland interface is defined by:
//! - A set of **requests** (client → server) with opcode + argument signature
//! - A set of **events** (server → client) with opcode + argument signature

pub mod wl_core;
pub mod wl_compositor;
pub mod wl_shm;
pub mod wl_output;
pub mod wl_seat;
pub mod wl_data_device;
pub mod wl_subcompositor;
pub mod xdg_shell;
pub mod xdg_decoration;
pub mod xdg_activation;
pub mod linux_dmabuf;
pub mod wp_viewporter;
pub mod wp_cursor_shape;
pub mod wp_presentation_time;
pub mod zxdg_idle_inhibit;
pub mod zwp_relative_pointer;
pub mod zwp_pointer_gestures;
pub mod zwp_text_input;
pub mod linux_drm_syncobj;
pub mod xdg_output;
pub mod xdg_foreign;
pub mod zwp_pointer_constraints;
pub mod screencopy;
pub mod text_input_v3;
pub mod input_method_v2;
pub mod primary_selection;
pub mod data_control;
pub mod dispatch;
pub mod edgerun_test_overlay;
