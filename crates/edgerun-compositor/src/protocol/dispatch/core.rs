//! Core protocol handlers: wl_display, wl_registry.

use super::{DispatchContext, GLOBALS};
use crate::wire;
use crate::wire::decode::ArgCursor;
use crate::wire::encode::*;
use edgerun_protocols::wayland::linux_dmabuf;
use edgerun_protocols::wayland::wl_core;
use edgerun_protocols::wayland::wl_seat;
use edgerun_protocols::wayland::wl_shm;

pub fn handle_display(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_core::display_request::SYNC => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let callback_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(callback_id, "wl_callback", 1, ctx.client_id);
            }
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(wl_core::callback_done_event(callback_id, 0));
            }
        }
        wl_core::display_request::GET_REGISTRY => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let registry_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(registry_id, "wl_registry", 1, ctx.client_id);
            }
            ctx.client_registry_ids.insert(ctx.client_id, registry_id);

            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                for (global_name_idx, (interface, version)) in GLOBALS.iter().enumerate() {
                    let name = (global_name_idx + 1) as u32;
                    let evt = wl_core::global_event(name, interface, *version);
                    client.send_message(evt);
                }
            }
        }
        _ => {}
    }
}

pub fn handle_registry(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_core::registry_request::BIND => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _name = cursor_obj.uint().unwrap_or(0);
            let interface_name = match cursor_obj.string() {
                Ok(Some(s)) => s.0,
                _ => return,
            };
            let version = cursor_obj.uint().unwrap_or(0);
            let id = cursor_obj.new_id().unwrap_or(0);

            eprintln!(
                "[edgerun-compositor] Client {} binding {} (v{}) -> id={}",
                ctx.client_id, interface_name, version, id
            );

            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(id, &interface_name, version, ctx.client_id);
            }

            match interface_name.as_str() {
                "wl_compositor" => {
                    ctx.client_compositor_ids.insert(ctx.client_id, id);
                }
                "wl_shm" => {
                    ctx.client_shm_ids.insert(ctx.client_id, id);
                    if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                        client.send_message(wl_shm::shm_format_event(id, wl_shm::format::XRGB8888));
                        client.send_message(wl_shm::shm_format_event(id, wl_shm::format::ARGB8888));
                        let _ = client.flush();
                    }
                }
                "wl_seat" => {
                    ctx.client_seat_ids.insert(ctx.client_id, id);
                    ctx.seat.id = id;
                    ctx.seat.capabilities = wl_seat::capability::KEYBOARD
                        | wl_seat::capability::POINTER
                        | wl_seat::capability::TOUCH;
                    if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                        client.send_message(wl_seat::seat_capabilities_event(
                            id,
                            ctx.seat.capabilities,
                        ));
                        if version >= 7 {
                            client.send_message(wl_seat::seat_name_event(id, "seat0"));
                        }
                        let _ = client.flush();
                    }
                }
                "xdg_wm_base" => {
                    ctx.client_xdg_base_ids.insert(ctx.client_id, id);
                    ctx.shell.base_id = id;
                }
                "wl_output" => {
                    ctx.client_output_ids.insert(ctx.client_id, id);
                    let actual_width = ctx.shell.output_width;
                    let actual_height = ctx.shell.output_height;
                    let actual_refresh = ctx.shell.output_refresh_mhz;
                    let mm_width = ctx.shell.output_mm_width;
                    let mm_height = ctx.shell.output_mm_height;
                    if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                        let mut geo_args = Vec::new();
                        geo_args.extend_from_slice(&0i32.to_le_bytes());
                        geo_args.extend_from_slice(&0i32.to_le_bytes());
                        geo_args.extend_from_slice(&mm_width.to_le_bytes());
                        geo_args.extend_from_slice(&mm_height.to_le_bytes());
                        geo_args.extend_from_slice(&0i32.to_le_bytes());
                        encode_string(&mut geo_args, "edgerun");
                        encode_string(&mut geo_args, "edgerun-output");
                        geo_args.extend_from_slice(&0i32.to_le_bytes());
                        client.send_message(wire::Message {
                            sender_id: id,
                            opcode: 0,
                            size: (8 + geo_args.len()) as u16,
                            args: geo_args,
                            fds: Vec::new(),
                        });
                        let mut mode_args = Vec::new();
                        mode_args.extend_from_slice(&3u32.to_le_bytes());
                        mode_args.extend_from_slice(&actual_width.to_le_bytes());
                        mode_args.extend_from_slice(&actual_height.to_le_bytes());
                        mode_args.extend_from_slice(&(actual_refresh).to_le_bytes());
                        client.send_message(wire::Message {
                            sender_id: id,
                            opcode: 1,
                            size: (8 + mode_args.len()) as u16,
                            args: mode_args,
                            fds: Vec::new(),
                        });
                        if version >= 3 {
                            let mut scale_args = Vec::new();
                            scale_args.extend_from_slice(&ctx.shell.output_scale.to_le_bytes());
                            client.send_message(wire::Message {
                                sender_id: id,
                                opcode: 3,
                                size: (8 + scale_args.len()) as u16,
                                args: scale_args,
                                fds: Vec::new(),
                            });
                        }
                        if version >= 4 {
                            let mut name_args = Vec::new();
                            encode_string(&mut name_args, "edgerun-output-0");
                            client.send_message(wire::Message {
                                sender_id: id,
                                opcode: 5,
                                size: (8 + name_args.len()) as u16,
                                args: name_args,
                                fds: Vec::new(),
                            });
                            // description (v4+)
                            let mut desc_args = Vec::new();
                            encode_string(&mut desc_args, "edgerun compositor output");
                            client.send_message(wire::Message {
                                sender_id: id,
                                opcode: 6,
                                size: (8 + desc_args.len()) as u16,
                                args: desc_args,
                                fds: Vec::new(),
                            });
                        }
                        if version >= 4 {
                            client.send_message(wire::Message {
                                sender_id: id,
                                opcode: 4,
                                size: 8,
                                args: Vec::new(),
                                fds: Vec::new(),
                            });
                        }
                        let _ = client.flush();
                    }
                }
                "zwp_linux_dmabuf_v1" => {
                    ctx.client_dmabuf_ids.insert(ctx.client_id, id);
                    if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                        for (format, modifier) in linux_dmabuf::common_formats_with_modifiers() {
                            let modifier_hi = (modifier >> 32) as u32;
                            let modifier_lo = (modifier & 0xFFFFFFFF) as u32;
                            client.send_message(linux_dmabuf::dmabuf_modifier_event(
                                id,
                                format,
                                modifier_hi,
                                modifier_lo,
                            ));
                        }
                        let _ = client.flush();
                    }
                }
                "wl_data_device_manager" => {}
                "wl_subcompositor" => {
                    ctx.client_subcompositor_ids.insert(ctx.client_id, id);
                }
                "zxdg_decoration_manager_v1" => {
                    ctx.client_decoration_manager_ids.insert(ctx.client_id, id);
                }
                "wp_viewporter" => {
                    ctx.client_viewporter_ids.insert(ctx.client_id, id);
                }
                "wp_cursor_shape_manager_v1" => {
                    ctx.client_cursor_shape_manager_ids
                        .insert(ctx.client_id, id);
                }
                "xdg_activation_v1" => {}
                "wp_presentation" => {}
                "zwp_relative_pointer_manager_v1" => {}
                "zwp_pointer_gestures_v1" => {}
                "zwp_text_input_manager_v1" => {}
                "zwp_idle_inhibit_manager_v1" => {}
                "zxdg_output_manager_v1" => {
                    ctx.client_output_ids.insert(ctx.client_id, id);
                }
                _ => {}
            }
        }
        _ => {}
    }
}
