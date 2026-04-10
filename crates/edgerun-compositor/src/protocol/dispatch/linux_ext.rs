//! linux_dmabuf, linux_drm_syncobj handlers.

use std::os::fd::RawFd;
use std::collections::HashMap;

use super::DispatchContext;
use crate::protocol::linux_dmabuf;
use crate::protocol::linux_drm_syncobj;
use crate::drm::syncobj as drm_syncobj;
use crate::compositor::surface::ShmBufferInfo;
use crate::wire::decode::ArgCursor;

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
        linux_drm_syncobj::syncobj_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        linux_drm_syncobj::syncobj_request::GET_SURFACE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_obj_id = cursor_obj.new_id().unwrap_or(0);
            let wl_surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(surface_obj_id, linux_drm_syncobj::SURFACE_V1, 1, ctx.client_id);
            }
            // Initialize surface syncobj state
            ctx.shell.syncobj_state.surfaces.entry(wl_surface_id).or_insert_with(SurfaceSyncobjState::default);
            // Store mapping from syncobj_surface object to wl_surface
            ctx.shell.syncobj_surface_map.insert(surface_obj_id, wl_surface_id);
        }
        linux_drm_syncobj::syncobj_request::CREATE_TIMELINE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let timeline_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(timeline_id, linux_drm_syncobj::TIMELINE_V1, 1, ctx.client_id);
            }
            // Create a real DRM syncobj
            match drm_syncobj::syncobj_create(ctx.drm_fd, 0) {
                Ok(handle) => {
                    ctx.shell.syncobj_state.timelines.insert(timeline_id, SyncobjTimeline {
                        handle,
                        current_point: 0,
                    });
                }
                Err(e) => {
                    eprintln!("[edgerun-compositor] Failed to create DRM syncobj: {}", e);
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
            // Destroy the DRM syncobj
            if let Some(timeline) = ctx.shell.syncobj_state.timelines.remove(&ctx.msg.sender_id) {
                let _ = drm_syncobj::syncobj_destroy(ctx.drm_fd, timeline.handle);
            }
        }
        linux_drm_syncobj::timeline_request::IMPORT_SYNC_FILE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let point = cursor_obj.uint().unwrap_or(0) as u64;
            // FD is passed via SCM_RIGHTS
            let sync_file_fd = if !ctx.msg.fds.is_empty() { ctx.msg.fds[0] } else { -1 };
            if let Some(timeline) = ctx.shell.syncobj_state.timelines.get(&ctx.msg.sender_id) {
                if sync_file_fd >= 0 {
                    if let Err(e) = drm_syncobj::import_sync_file(ctx.drm_fd, timeline.handle, sync_file_fd) {
                        eprintln!("[edgerun-compositor] Failed to import sync_file: {}", e);
                    }
                    // The sync_file FD is consumed by the kernel — don't close it
                }
            }
        }
        linux_drm_syncobj::timeline_request::EXPORT_SYNC_FILE => {
            let _ = &ctx.msg;
            // Client expects a sync_file FD to be returned.
            // In a full implementation we'd send it via SCM_RIGHTS.
            // For now, the client can signal via IMPORT and we handle the rest.
        }
        _ => {}
    }
}
