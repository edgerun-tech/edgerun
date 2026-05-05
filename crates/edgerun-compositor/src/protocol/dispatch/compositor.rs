//! wl_compositor, wl_surface, wl_region handlers.

use super::DispatchContext;
use crate::compositor::surface::{DamageRect, SurfaceBuffer};
use crate::protocol::wl_compositor;
use crate::protocol::wp_presentation_time;
use crate::wire::decode::ArgCursor;

/// Per-client region state — maps region object id to its rectangles.
/// Stored in main.rs and passed through DispatchContext.
pub struct RegionRegistry {
    pub regions: std::collections::HashMap<u32, Vec<DamageRect>>,
}

impl RegionRegistry {
    pub fn new() -> Self {
        Self {
            regions: std::collections::HashMap::new(),
        }
    }

    pub fn set_rects(&mut self, region_id: u32, rects: Vec<DamageRect>) {
        self.regions.insert(region_id, rects);
    }

    pub fn get_rects(&self, region_id: u32) -> Option<&Vec<DamageRect>> {
        self.regions.get(&region_id)
    }

    pub fn remove(&mut self, region_id: u32) {
        self.regions.remove(&region_id);
    }
}

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
                ctx.surfaces
                    .attach(ctx.msg.sender_id, SurfaceBuffer::Null, x, y);
            } else if let Some(info) = ctx.buffers.get(buffer_id) {
                ctx.surfaces.attach(
                    ctx.msg.sender_id,
                    SurfaceBuffer::Shm {
                        pool_fd: info.pool_fd,
                        offset: info.offset,
                        width: info.width,
                        height: info.height,
                        stride: info.stride,
                        format: info.format,
                    },
                    x,
                    y,
                );
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
            ctx.surfaces
                .add_frame_callback(ctx.msg.sender_id, callback_id);
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
                                ctx.msg.sender_id,
                                output_id,
                            ));
                        }
                        if let Some(client) = ctx.server.client_mut(cid) {
                            client.send_message(
                                wl_compositor::surface_preferred_buffer_scale_event(
                                    ctx.msg.sender_id,
                                    surface.buffer_scale as u32,
                                ),
                            );
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
            ctx.surfaces
                .set_buffer_transform(ctx.msg.sender_id, transform);
        }
        wl_compositor::surface_request::SET_OPAQUE_REGION => {
            let region_id = ArgCursor::from_message(&ctx.msg).object().unwrap_or(0);
            if let Some(surface) = ctx.surfaces.get_mut(ctx.msg.sender_id) {
                if region_id != 0 {
                    // Copy rects from the region object via region registry
                    let rects = ctx
                        .region_registry
                        .get_rects(region_id)
                        .cloned()
                        .unwrap_or_default();
                    surface.opaque = true;
                    surface.opaque_region = rects;
                } else {
                    surface.opaque = false;
                    surface.opaque_region.clear();
                }
            }
        }
        wl_compositor::surface_request::SET_INPUT_REGION => {
            let region_id = ArgCursor::from_message(&ctx.msg).object().unwrap_or(0);
            if let Some(surface) = ctx.surfaces.get_mut(ctx.msg.sender_id) {
                if region_id != 0 {
                    // Copy rects from the region object via region registry
                    let rects = ctx
                        .region_registry
                        .get_rects(region_id)
                        .cloned()
                        .unwrap_or_default();
                    surface.input_region = Some(rects);
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
            let scale = ctx
                .surfaces
                .get(ctx.msg.sender_id)
                .map(|s| s.buffer_scale)
                .unwrap_or(1);
            ctx.surfaces.damage(
                ctx.msg.sender_id,
                x * scale,
                y * scale,
                w * scale,
                h * scale,
            );
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
                                ctx.msg.sender_id,
                                output_id,
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
    let region_id = ctx.msg.sender_id;
    match ctx.msg.opcode {
        wl_compositor::region_request::DESTROY => {
            ctx.region_registry.remove(region_id);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(region_id);
            }
        }
        wl_compositor::region_request::ADD => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            let rect = DamageRect {
                x,
                y,
                width: w,
                height: h,
            };
            // Get current rects for this region, or start empty
            let mut rects = ctx
                .region_registry
                .get_rects(region_id)
                .cloned()
                .unwrap_or_default();
            rects.push(rect);
            ctx.region_registry.set_rects(region_id, rects);
        }
        wl_compositor::region_request::SUBTRACT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            let subtract_rect = DamageRect {
                x,
                y,
                width: w,
                height: h,
            };
            // Get current rects and subtract
            let mut rects = ctx
                .region_registry
                .get_rects(region_id)
                .cloned()
                .unwrap_or_default();
            rects.retain(|r| {
                // Simple rect removal — remove any rect that overlaps with subtract_rect
                !(r.x < subtract_rect.x + subtract_rect.width
                    && r.x + r.width > subtract_rect.x
                    && r.y < subtract_rect.y + subtract_rect.height
                    && r.y + r.height > subtract_rect.y)
            });
            ctx.region_registry.set_rects(region_id, rects);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_registry_create_add_destroy() {
        let mut reg = RegionRegistry::new();
        let region_id = 42;

        // Initially no rects
        assert!(reg.get_rects(region_id).is_none());

        // Add a rect
        reg.set_rects(
            region_id,
            vec![DamageRect {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
            }],
        );
        let rects = reg.get_rects(region_id).unwrap();
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].width, 100);

        // Destroy removes the region
        reg.remove(region_id);
        assert!(reg.get_rects(region_id).is_none());
    }

    #[test]
    fn test_region_registry_multiple_rects() {
        let mut reg = RegionRegistry::new();
        let region_id = 1;

        reg.set_rects(
            region_id,
            vec![
                DamageRect {
                    x: 0,
                    y: 0,
                    width: 50,
                    height: 50,
                },
                DamageRect {
                    x: 60,
                    y: 60,
                    width: 40,
                    height: 40,
                },
            ],
        );
        assert_eq!(reg.get_rects(region_id).unwrap().len(), 2);
    }

    #[test]
    fn test_region_subtract_removes_overlap() {
        let mut reg = RegionRegistry::new();
        let region_id = 1;

        reg.set_rects(
            region_id,
            vec![
                DamageRect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
                DamageRect {
                    x: 200,
                    y: 200,
                    width: 50,
                    height: 50,
                }, // non-overlapping
            ],
        );

        // Subtract a rect that overlaps the first one
        let subtract = DamageRect {
            x: 0,
            y: 0,
            width: 50,
            height: 50,
        };
        let mut rects = reg.get_rects(region_id).unwrap().clone();
        rects.retain(|r| {
            !(r.x < subtract.x + subtract.width
                && r.x + r.width > subtract.x
                && r.y < subtract.y + subtract.height
                && r.y + r.height > subtract.y)
        });
        reg.set_rects(region_id, rects);

        // First rect was overlapping and removed, second remains
        assert_eq!(reg.get_rects(region_id).unwrap().len(), 1);
    }

    #[test]
    fn test_two_regions_independent() {
        let mut reg = RegionRegistry::new();
        reg.set_rects(
            1,
            vec![DamageRect {
                x: 0,
                y: 0,
                width: 10,
                height: 10,
            }],
        );
        reg.set_rects(
            2,
            vec![DamageRect {
                x: 100,
                y: 100,
                width: 20,
                height: 20,
            }],
        );

        assert_eq!(reg.get_rects(1).unwrap()[0].width, 10);
        assert_eq!(reg.get_rects(2).unwrap()[0].width, 20);
    }
}
