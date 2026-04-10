//! linux_dmabuf, linux_drm_syncobj handlers.

use super::DispatchContext;
use crate::protocol::linux_dmabuf;
use crate::protocol::linux_drm_syncobj;
use crate::compositor::surface::ShmBufferInfo;
use crate::wire::decode::ArgCursor;

pub fn handle_dmabuf(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        linux_dmabuf::dmabuf_request::DESTROY => {}
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
            let _wl_surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(surface_obj_id, linux_drm_syncobj::SURFACE_V1, 1, ctx.client_id);
            }
        }
        linux_drm_syncobj::syncobj_request::CREATE_TIMELINE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let timeline_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(timeline_id, linux_drm_syncobj::TIMELINE_V1, 1, ctx.client_id);
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
        }
        linux_drm_syncobj::surface_request::SET_ACQUIRE_POINT |
        linux_drm_syncobj::surface_request::SET_RELEASE_POINT => {
            // Stub: accept but ignore sync points
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
        }
        linux_drm_syncobj::timeline_request::IMPORT_SYNC_FILE |
        linux_drm_syncobj::timeline_request::EXPORT_SYNC_FILE => {
            // Stub: accept but ignore sync file operations
        }
        _ => {}
    }
}
