//! wl_data_device_manager, wl_data_device, wl_data_source, wl_data_offer handlers.

use super::{DataSource, DispatchContext};
use crate::protocol::wl_data_device;
use crate::wire::decode::ArgCursor;

pub fn handle_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_data_device::dnd_manager_request::CREATE_DATA_SOURCE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let source_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(source_id, "wl_data_source", 3, ctx.client_id);
            }
            ctx.client_data_source_ids.insert(ctx.client_id, source_id);
        }
        wl_data_device::dnd_manager_request::GET_DATA_DEVICE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let device_id = cursor_obj.new_id().unwrap_or(0);
            let _seat_obj_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(device_id, "wl_data_device", 3, ctx.client_id);
            }
            ctx.client_data_device_ids.insert(ctx.client_id, device_id);
        }
        wl_data_device::dnd_manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_source(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_data_device::data_source_request::OFFER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(mime_type)) = cursor_obj.string() {
                if let Some(source) = ctx.current_data_source.as_mut() {
                    source.mime_types.push(mime_type.0);
                }
            }
        }
        wl_data_device::data_source_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_data_source_ids.remove(&ctx.client_id);
        }
        wl_data_device::data_source_request::SET_ACTIONS => {}
        _ => {}
    }
}

pub fn handle_offer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_data_device::data_offer_request::ACCEPT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _serial = cursor_obj.uint().unwrap_or(0);
            let _mime_type = cursor_obj.string().ok().flatten();
        }
        wl_data_device::data_offer_request::RECEIVE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(mime_type)) = cursor_obj.string() {
                let fd = if !ctx.msg.fds.is_empty() {
                    ctx.msg.fds[0]
                } else {
                    -1
                };
                if fd >= 0 {
                    if let Some(ref source) = *ctx.current_data_source {
                        if let Some(source_client) = ctx.server.client_mut(source.owner_client_id) {
                            source_client.send_message(wl_data_device::data_source_send_event(
                                source.id,
                                &mime_type.0,
                                fd,
                            ));
                        }
                    }
                }
            }
        }
        wl_data_device::data_offer_request::DISCRIPTION => {}
        wl_data_device::data_offer_request::SET_ACTIONS => {}
        wl_data_device::data_offer_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_device(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_data_device::data_device_request::SET_SELECTION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let source_id = cursor_obj.object().unwrap_or(0);
            let _serial = cursor_obj.uint().unwrap_or(0);

            if source_id != 0 {
                *ctx.selection_offer_counter += 1;
                let offer_id = *ctx.selection_offer_counter;

                *ctx.current_data_source = Some(DataSource {
                    id: source_id,
                    owner_client_id: ctx.client_id,
                    mime_types: Vec::new(),
                });

                for (&cid, &device_id) in ctx.client_data_device_ids.iter() {
                    if let Some(client) = ctx.server.client_mut(cid) {
                        client.send_message(wl_data_device::data_device_data_offer_event(
                            device_id, offer_id,
                        ));
                        client.send_message(wl_data_device::data_device_selection_event(
                            device_id, offer_id,
                        ));
                        if let Some(ref source) = *ctx.current_data_source {
                            for mime in &source.mime_types {
                                client.send_message(wl_data_device::data_offer_offer_event(
                                    offer_id, mime,
                                ));
                            }
                        }
                    }
                }
            } else {
                *ctx.current_data_source = None;
                for (&cid, &device_id) in ctx.client_data_device_ids.iter() {
                    if let Some(client) = ctx.server.client_mut(cid) {
                        client.send_message(wl_data_device::data_device_selection_event(
                            device_id, 0,
                        ));
                    }
                }
            }
        }
        wl_data_device::data_device_request::START_DRAG => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let source_id = cursor_obj.object().unwrap_or(0);
            let origin_surface_id = cursor_obj.object().unwrap_or(0);
            let icon_id = cursor_obj.object().unwrap_or(0);
            let _serial = cursor_obj.uint().unwrap_or(0);
            let _ = (origin_surface_id, icon_id);

            if source_id != 0 {
                // Create a drag data offer
                *ctx.selection_offer_counter += 1;
                let offer_id = *ctx.selection_offer_counter;

                *ctx.current_data_source = Some(DataSource {
                    id: source_id,
                    owner_client_id: ctx.client_id,
                    mime_types: Vec::new(),
                });

                // Broadcast drag enter to the target surface
                if let Some(surface) = ctx.surfaces.get(origin_surface_id) {
                    for (&cid, &device_id) in ctx.client_data_device_ids.iter() {
                        if let Some(client) = ctx.server.client_mut(cid) {
                            client.send_message(wl_data_device::data_device_data_offer_event(
                                device_id, offer_id,
                            ));
                            client.send_message(wl_data_device::data_device_enter_event(
                                device_id,
                                ctx.seat.next_serial(),
                                origin_surface_id,
                                surface.x as f64,
                                surface.y as f64,
                                offer_id,
                            ));
                            if let Some(ref source) = *ctx.current_data_source {
                                for mime in &source.mime_types {
                                    client.send_message(wl_data_device::data_offer_offer_event(
                                        offer_id, mime,
                                    ));
                                }
                            }
                        }
                    }
                    eprintln!(
                        "[edgerun-compositor] Drag started from surface {}",
                        origin_surface_id
                    );
                }
            }
        }
        wl_data_device::data_device_request::RELEASE => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}
