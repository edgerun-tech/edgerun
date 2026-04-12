//! linux_dmabuf, linux_drm_syncobj handlers.

use std::os::fd::RawFd;
use std::collections::HashMap;

use super::DispatchContext;
use crate::protocol::linux_dmabuf;
use crate::protocol::linux_drm_syncobj;
use crate::drm::syncobj as drm_syncobj;
use crate::compositor::surface::ShmBufferInfo;
use crate::wire::decode::ArgCursor;

/// Send a protocol error to the client and mark them disconnected.
fn client_error(ctx: &mut DispatchContext, code: u32, msg: &str) {
    eprintln!("[edgerun-compositor] Protocol error (code={code}): {msg}");
    if let Some(client) = ctx.server.client_mut(ctx.client_id) {
        client.disconnected = true;
    }
}

/// Per-surface syncobj state — tracks acquire/release fences.
#[derive(Debug, Default)]
pub struct SurfaceSyncobjState {
    /// Syncobj handle for acquire (client signals when buffer is ready).
    pub acquire_handle: Option<u32>,
    /// Timeline point for acquire (monotonic counter).
    pub acquire_point: u64,
    /// Syncobj handle for release (compositor signals when done with buffer).
    pub release_handle: Option<u32>,
    /// Timeline point for release.
    pub release_point: u64,
}

/// Per-timeline syncobj state.
#[derive(Debug)]
pub struct SyncobjTimeline {
    /// DRM syncobj handle.
    pub handle: u32,
    /// Current signaled point on this timeline.
    pub current_point: u64,
}

/// Global syncobj state — shared across all dispatches.
pub struct SyncobjState {
    /// Per-surface syncobj state.
    pub surfaces: HashMap<u32, SurfaceSyncobjState>,
    /// Per-timeline syncobj handles keyed by wl object id.
    pub timelines: HashMap<u32, SyncobjTimeline>,
}

impl SyncobjState {
    pub fn new() -> Self {
        Self {
            surfaces: HashMap::new(),
            timelines: HashMap::new(),
        }
    }

    /// Wait for a surface's acquire fence to be ready before compositing.
    pub fn wait_acquire(&self, drm_fd: RawFd, surface_id: u32) {
        if let Some(state) = self.surfaces.get(&surface_id) {
            if let Some(handle) = state.acquire_handle {
                if state.acquire_point > 0 {
                    // Non-blocking check — if not ready, we'll wait next frame
                    let _ = drm_syncobj::timeline_wait(
                        drm_fd, handle, state.acquire_point,
                        0 /* non-blocking */, 0,
                    );
                }
            }
        }
    }

    /// Signal the release fence for a surface after page flip completes.
    pub fn signal_release(&self, drm_fd: RawFd, surface_id: u32) {
        if let Some(state) = self.surfaces.get(&surface_id) {
            if let Some(handle) = state.release_handle {
                // Signal the release timeline to the current point
                let _ = drm_syncobj::timeline_signal(drm_fd, handle, state.release_point);
            }
        }
    }
}

pub fn handle_dmabuf(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        linux_dmabuf::dmabuf_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        linux_dmabuf::dmabuf_request::CREATE_PARAMS => {
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(linux_dmabuf::dmabuf_format_event(ctx.msg.sender_id, linux_dmabuf::dmabuf_format::XRGB8888));
                client.send_message(linux_dmabuf::dmabuf_format_event(ctx.msg.sender_id, linux_dmabuf::dmabuf_format::ARGB8888));
                client.send_message(linux_dmabuf::dmabuf_modifier_event(
                    ctx.msg.sender_id, linux_dmabuf::dmabuf_format::XRGB8888,
                    (linux_dmabuf::DRM_FORMAT_MOD_LINEAR >> 32) as u32,
                    linux_dmabuf::DRM_FORMAT_MOD_LINEAR as u32,
                ));
            }
        }
        linux_dmabuf::dmabuf_request::CREATE_IMMED => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let buffer_id = cursor_obj.new_id().unwrap_or(0);
            let fd = if !ctx.msg.fds.is_empty() { ctx.msg.fds[0] } else { -1 };
            let width = cursor_obj.int().unwrap_or(0);
            let height = cursor_obj.int().unwrap_or(0);
            let stride = cursor_obj.int().unwrap_or(0);
            let format = cursor_obj.uint().unwrap_or(0);
            ctx.buffers.register(buffer_id, ShmBufferInfo {
                pool_fd: fd, offset: 0, width, height, stride, format,
            });
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(buffer_id, "wl_buffer", 1, ctx.client_id);
            }
        }
        _ => {}
    }
}

pub fn handle_syncobj(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        linux_drm_syncobj::manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        linux_drm_syncobj::manager_request::GET_SURFACE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_obj_id = cursor_obj.new_id().unwrap_or(0);
            let wl_surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                // Check if surface already has a syncobj surface — raise SURFACE_EXISTS
                if ctx.shell.syncobj_surface_map.values().any(|&s| s == wl_surface_id) {
                    client_error(ctx, linux_drm_syncobj::error::SURFACE_EXISTS,
                        "surface already has a syncobj surface");
                    return;
                }
                reg.register(surface_obj_id, linux_drm_syncobj::SURFACE_V1, 1, ctx.client_id);
            }
            // Initialize surface syncobj state
            ctx.shell.syncobj_state.surfaces.entry(wl_surface_id).or_insert_with(SurfaceSyncobjState::default);
            // Store mapping from syncobj_surface object to wl_surface
            ctx.shell.syncobj_surface_map.insert(surface_obj_id, wl_surface_id);
        }
        linux_drm_syncobj::manager_request::IMPORT_TIMELINE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let timeline_id = cursor_obj.new_id().unwrap_or(0);
            let syncobj_fd = if !ctx.msg.fds.is_empty() { ctx.msg.fds[0] } else { -1 };
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(timeline_id, linux_drm_syncobj::TIMELINE_V1, 1, ctx.client_id);
            }
            if syncobj_fd < 0 {
                client_error(ctx, linux_drm_syncobj::error::INVALID_TIMELINE,
                    "import_timeline: no FD provided");
                return;
            }
            // Import the DRM syncobj FD into a kernel handle
            match drm_syncobj::syncobj_import(ctx.drm_fd, syncobj_fd) {
                Ok(handle) => {
                    ctx.shell.syncobj_state.timelines.insert(timeline_id, SyncobjTimeline {
                        handle,
                        current_point: 0,
                    });
                }
                Err(e) => {
                    eprintln!("[edgerun-compositor] Failed to import syncobj timeline: {}", e);
                    client_error(ctx, linux_drm_syncobj::error::INVALID_TIMELINE,
                        "failed to import syncobj FD");
                }
            }
        }
        _ => {}
    }
}

pub fn handle_syncobj_surface(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        linux_drm_syncobj::surface_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.shell.syncobj_surface_map.remove(&ctx.msg.sender_id);
        }
        linux_drm_syncobj::surface_request::SET_ACQUIRE_POINT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let timeline_id = cursor_obj.object().unwrap_or(0);
            let point = cursor_obj.uint().unwrap_or(0) as u64;
            // Map the syncobj_surface object to a wl_surface
            if let Some(&surface_id) = ctx.shell.syncobj_surface_map.get(&ctx.msg.sender_id) {
                if let Some(timeline) = ctx.shell.syncobj_state.timelines.get(&timeline_id) {
                    if let Some(state) = ctx.shell.syncobj_state.surfaces.get_mut(&surface_id) {
                        state.acquire_handle = Some(timeline.handle);
                        state.acquire_point = point;
                    }
                }
            }
        }
        linux_drm_syncobj::surface_request::SET_RELEASE_POINT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let timeline_id = cursor_obj.object().unwrap_or(0);
            let point = cursor_obj.uint().unwrap_or(0) as u64;
            if let Some(&surface_id) = ctx.shell.syncobj_surface_map.get(&ctx.msg.sender_id) {
                if let Some(timeline) = ctx.shell.syncobj_state.timelines.get(&timeline_id) {
                    if let Some(state) = ctx.shell.syncobj_state.surfaces.get_mut(&surface_id) {
                        state.release_handle = Some(timeline.handle);
                        state.release_point = point;
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn handle_syncobj_timeline(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        linux_drm_syncobj::timeline_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            // Destroy the DRM syncobj handle (FD was consumed during import)
            if let Some(timeline) = ctx.shell.syncobj_state.timelines.remove(&ctx.msg.sender_id) {
                let _ = drm_syncobj::syncobj_destroy(ctx.drm_fd, timeline.handle);
            }
        }
        _ => {}
    }
}
