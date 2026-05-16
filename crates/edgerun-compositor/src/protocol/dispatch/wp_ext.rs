//! wp_* (viewporter, cursor_shape, presentation, single_pixel_buffer,
//! fractional_scale, tearing_control) handlers.

use super::DispatchContext;
use crate::compositor::surface::ShmBufferInfo;
use crate::libc;
use edgerun_protocols::wayland::decode::ArgCursor;
use edgerun_protocols::wayland::fractional_scale;
use edgerun_protocols::wayland::single_pixel_buffer;
use edgerun_protocols::wayland::tearing_control;
use edgerun_protocols::wayland::wl_shm;
use edgerun_protocols::wayland::wp_cursor_shape;
use edgerun_protocols::wayland::wp_presentation_time;
use edgerun_protocols::wayland::wp_viewporter;

pub fn handle_viewporter(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wp_viewporter::viewporter_request::GET_VIEWPORT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let viewport_id = cursor_obj.new_id().unwrap_or(0);
            let surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(viewport_id, "wp_viewport", 1, ctx.client_id);
            }
            ctx.client_viewporter_ids.insert(viewport_id, surface_id);
        }
        wp_viewporter::viewporter_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_viewport(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wp_viewporter::viewport_request::SET_SOURCE => {
            let Some(surface_id) = ctx.client_viewporter_ids.get(&ctx.msg.sender_id).copied()
            else {
                return;
            };
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
            let y = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
            let w = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
            let h = wp_viewporter::fixed_to_f64(cursor_obj.fixed().unwrap_or(0));
            ctx.surfaces.set_viewport_source(surface_id, x, y, w, h);
        }
        wp_viewporter::viewport_request::SET_DESTINATION => {
            let Some(surface_id) = ctx.client_viewporter_ids.get(&ctx.msg.sender_id).copied()
            else {
                return;
            };
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let width = cursor_obj.int().unwrap_or(-1);
            let height = cursor_obj.int().unwrap_or(-1);
            ctx.surfaces
                .set_viewport_destination(surface_id, width, height);
        }
        wp_viewporter::viewport_request::DESTROY => {
            if let Some(surface_id) = ctx.client_viewporter_ids.remove(&ctx.msg.sender_id) {
                ctx.surfaces
                    .set_viewport_source(surface_id, 0.0, 0.0, 0.0, 0.0);
                ctx.surfaces.set_viewport_destination(surface_id, -1, -1);
            }
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_cursor_shape_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wp_cursor_shape::cursor_shape_manager_request::GET_POINTER_SHAPE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let device_id = cursor_obj.new_id().unwrap_or(0);
            let _serial = cursor_obj.uint().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(device_id, "wp_cursor_shape_device_v1", 1, ctx.client_id);
            }
            ctx.client_cursor_shape_device_ids
                .insert(ctx.client_id, device_id);
        }
        wp_cursor_shape::cursor_shape_manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_cursor_shape_device(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wp_cursor_shape::cursor_shape_device_request::SET_SHAPE => {
            let shape_id = ArgCursor::from_message(&ctx.msg).uint().unwrap_or(0);
            ctx.cursor
                .set_shape(crate::render::cursor::shape_id_to_index(shape_id));
        }
        wp_cursor_shape::cursor_shape_device_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_cursor_shape_device_ids.remove(&ctx.client_id);
        }
        _ => {}
    }
}

pub fn handle_presentation(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wp_presentation_time::presentation_request::FEEDBACK => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_id = cursor_obj.object().unwrap_or(0);
            let feedback_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(feedback_id, "wp_presentation_feedback", 1, ctx.client_id);
            }
            ctx.presentation_tracker
                .register(surface_id, feedback_id, ctx.client_id);
        }
        wp_presentation_time::presentation_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_presentation_feedback(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wp_presentation_time::presentation_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_single_pixel_buffer_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        single_pixel_buffer::manager_request::CREATE_SRGB32_BUFFER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let buffer_id = cursor_obj.new_id().unwrap_or(0);
            let red = cursor_obj.uint().unwrap_or(0);
            let green = cursor_obj.uint().unwrap_or(0);
            let blue = cursor_obj.uint().unwrap_or(0);
            let alpha = cursor_obj.uint().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(
                    buffer_id,
                    single_pixel_buffer::WP_SINGLE_PIXEL_BUFFER_V1,
                    1,
                    ctx.client_id,
                );
            }
            let fd = unsafe {
                libc::memfd_create(b"edgerun-single-pixel\0".as_ptr() as *const libc::c_char, 0)
            };
            if fd >= 0 {
                unsafe { libc::ftruncate(fd, 4) };
                let pixel: [u8; 4] = [red as u8, green as u8, blue as u8, alpha as u8];
                unsafe { libc::pwrite(fd, pixel.as_ptr() as *const libc::c_void, 4, 0) };
                ctx.buffers.register(
                    buffer_id,
                    ShmBufferInfo {
                        pool_fd: fd,
                        offset: 0,
                        width: 1,
                        height: 1,
                        stride: 4,
                        format: wl_shm::format::XRGB8888,
                    },
                );
                eprintln!(
                    "[edgerun-compositor] Single pixel buffer created: id={} rgba=({},{},{},{})",
                    buffer_id, red, green, blue, alpha
                );
            } else {
                eprintln!("[edgerun-compositor] Failed to create memfd for single pixel buffer");
            }
        }
        single_pixel_buffer::manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_single_pixel_buffer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        single_pixel_buffer::buffer_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.buffers.remove(ctx.msg.sender_id);
        }
        _ => {}
    }
}

pub fn handle_fractional_scale_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        fractional_scale::manager_request::GET_FRACTIONAL_SCALE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let scale_id = cursor_obj.new_id().unwrap_or(0);
            let _surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(
                    scale_id,
                    fractional_scale::WP_FRACTIONAL_SCALE_V1,
                    1,
                    ctx.client_id,
                );
            }
            let scale_value = (ctx.shell.output_scale * 120) as u32;
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(fractional_scale::preferred_scale_event(
                    scale_id,
                    scale_value,
                ));
                let _ = client.flush();
            }
        }
        fractional_scale::manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_fractional_scale(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        fractional_scale::fractional_scale_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_tearing_control_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        tearing_control::manager_request::GET_TEARING_CONTROL => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let control_id = cursor_obj.new_id().unwrap_or(0);
            let surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(
                    control_id,
                    tearing_control::WP_TEARING_CONTROL_V1,
                    1,
                    ctx.client_id,
                );
            }
            ctx.client_tearing_control_ids
                .insert(control_id, surface_id);
        }
        tearing_control::manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_tearing_control(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        tearing_control::tearing_control_request::SET_PRESENTATION_HINT => {
            let hint = ArgCursor::from_message(&ctx.msg).uint().unwrap_or(0);
            if let Some(&surface_id) = ctx.client_tearing_control_ids.get(&ctx.msg.sender_id) {
                ctx.surfaces.set_tearing_hint(surface_id, hint);
            }
            match hint {
                tearing_control::hint::DEFAULT => {
                    eprintln!(
                        "[edgerun-compositor] Tearing control: default (surface {})",
                        ctx.msg.sender_id
                    );
                }
                tearing_control::hint::SYNC => {
                    eprintln!(
                        "[edgerun-compositor] Tearing control: sync/VSync (surface {})",
                        ctx.msg.sender_id
                    );
                }
                tearing_control::hint::ASYNC => {
                    eprintln!(
                        "[edgerun-compositor] Tearing control: async/tearing allowed (surface {})",
                        ctx.msg.sender_id
                    );
                }
                _ => {}
            }
        }
        tearing_control::tearing_control_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_tearing_control_ids.remove(&ctx.msg.sender_id);
        }
        _ => {}
    }
}
