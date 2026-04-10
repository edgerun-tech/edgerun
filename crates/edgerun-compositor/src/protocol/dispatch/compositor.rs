//! wl_compositor, wl_surface, wl_region handlers.

use super::DispatchContext;
use crate::compositor::surface::{SurfaceBuffer, DamageRect};
use crate::protocol::wl_compositor;
use crate::protocol::wp_presentation_time;
use crate::wire::decode::ArgCursor;

pub fn handle_compositor(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_compositor::compositor_request::CREATE_SURFACE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(surface_id, "wl_surface", 4, ctx.client_id);
            }
            ctx.surfaces.create(surface_id);
        }
        wl_compositor::compositor_request::CREATE_REGION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let region_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(region_id, "wl_region", 1, ctx.client_id);
            }
        }
        _ => {}
    }
}

pub fn handle_surface(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_compositor::surface_request::ATTACH => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let buffer_id = cursor_obj.object().unwrap_or(0);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);

            if buffer_id == 0 {
                ctx.surfaces.attach(ctx.msg.sender_id, SurfaceBuffer::Null, x, y);
            } else if let Some(info) = ctx.buffers.get(buffer_id) {
                ctx.surfaces.attach(ctx.msg.sender_id, SurfaceBuffer::Shm {
                    pool_fd: info.pool_fd,
                    offset: info.offset,
                    width: info.width,
                    height: info.height,
                    stride: info.stride,
                    format: info.format,
                }, x, y);
            }
        }
        wl_compositor::surface_request::DAMAGE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            ctx.surfaces.damage(ctx.msg.sender_id, x, y, w, h);
        }
        wl_compositor::surface_request::FRAME => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let callback_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(callback_id, "wl_callback", 1, ctx.client_id);
            }
            ctx.surfaces.add_frame_callback(ctx.msg.sender_id, callback_id);
        }
        wl_compositor::surface_request::COMMIT => {
            for (feedback_id, client_id) in ctx.presentation_tracker.supersede(ctx.msg.sender_id) {
                if let Some(client) = ctx.server.client_mut(client_id) {
                    client.send_message(wp_presentation_time::feedback_discarded_event(
                        feedback_id,
                        wp_presentation_time::discard_reason::SUPERSEDED,
                    ));
                }
            }
            ctx.surfaces.commit(ctx.msg.sender_id);

            if let Some(surface) = ctx.surfaces.get(ctx.msg.sender_id) {
                if surface.buffer.is_some() {
                    for (&cid, &output_id) in ctx.client_output_ids.iter() {
                        if let Some(client) = ctx.server.client_mut(cid) {
                            client.send_message(wl_compositor::surface_enter_event(
                                ctx.msg.sender_id, output_id,
                            ));
                        }
                        if let Some(client) = ctx.server.client_mut(cid) {
                            client.send_message(wl_compositor::surface_preferred_buffer_scale_event(
                                ctx.msg.sender_id, surface.buffer_scale as u32,
                            ));
                        }
                    }
                }
            }
        }
        wl_compositor::surface_request::SET_BUFFER_SCALE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let scale = cursor_obj.int().unwrap_or(1);
            ctx.surfaces.set_buffer_scale(ctx.msg.sender_id, scale);
        }
        wl_compositor::surface_request::SET_BUFFER_TRANSFORM => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let transform = cursor_obj.int().unwrap_or(0);
            ctx.surfaces.set_buffer_transform(ctx.msg.sender_id, transform);
        }
        wl_compositor::surface_request::SET_OPAQUE_REGION => {
            let region_id = ArgCursor::from_message(&ctx.msg).object().unwrap_or(0);
            if let Some(surface) = ctx.surfaces.get_mut(ctx.msg.sender_id) {
                surface.opaque = region_id != 0;
            }
        }
        wl_compositor::surface_request::SET_INPUT_REGION => {
            let region_id = ArgCursor::from_message(&ctx.msg).object().unwrap_or(0);
            if let Some(surface) = ctx.surfaces.get_mut(ctx.msg.sender_id) {
                if region_id != 0 {
                    // Copy region rects from the region object (tracked per-surface)
                    surface.input_region = Some(surface.opaque_region.clone());
                } else {
                    surface.input_region = None;
                }
            }
        }
        wl_compositor::surface_request::DAMAGE_BUFFER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            let scale = ctx.surfaces.get(ctx.msg.sender_id).map(|s| s.buffer_scale).unwrap_or(1);
            ctx.surfaces.damage(ctx.msg.sender_id, x * scale, y * scale, w * scale, h * scale);
        }
        wl_compositor::surface_request::OFFSET => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            if let Some(s) = ctx.surfaces.get_mut(ctx.msg.sender_id) {
                s.x = x;
                s.y = y;
            }
        }
        wl_compositor::surface_request::DESTROY => {
            if let Some(surface) = ctx.surfaces.get(ctx.msg.sender_id) {
                if surface.buffer.is_some() {
                    for (&cid, &output_id) in ctx.client_output_ids.iter() {
                        if let Some(client) = ctx.server.client_mut(cid) {
                            client.send_message(wl_compositor::surface_leave_event(
                                ctx.msg.sender_id, output_id,
                            ));
                        }
                    }
                }
            }
            ctx.surfaces.destroy(ctx.msg.sender_id);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            super::send_delete_id(ctx.server, ctx.client_id, ctx.msg.sender_id);
        }
        _ => {}
    }
}

pub fn handle_region(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_compositor::region_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        wl_compositor::region_request::ADD => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            let rect = DamageRect { x, y, width: w, height: h };
            let ids: Vec<u32> = ctx.surfaces.surfaces()
                .filter(|s| s.opaque)
                .map(|s| s.id)
                .collect();
            for id in ids {
                ctx.surfaces.add_opaque_region_rect(id, rect);
            }
        }
        wl_compositor::region_request::SUBTRACT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            let rect = DamageRect { x, y, width: w, height: h };
            let ids: Vec<u32> = ctx.surfaces.surfaces()
                .filter(|s| s.opaque)
                .map(|s| s.id)
                .collect();
            for id in ids {
                ctx.surfaces.subtract_opaque_region_rect(id, rect);
            }
        }
        _ => {}
    }
}
