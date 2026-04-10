//! xdg_wm_base, xdg_surface, xdg_toplevel, xdg_positioner, xdg_popup handlers.

use super::DispatchContext;
use crate::protocol::xdg_shell;
use crate::protocol::wl_seat;
use crate::wire::decode::ArgCursor;

pub fn handle_wm_base(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_shell::xdg_wm_base_request::GET_XDG_SURFACE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let xdg_surface_id = cursor_obj.new_id().unwrap_or(0);
            let wl_surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(xdg_surface_id, "xdg_surface", 6, ctx.client_id);
            }
            ctx.xdg_surface_to_wl_surface.insert(xdg_surface_id, wl_surface_id);
            println!("[edgerun-compositor] xdg_surface created: id={} wl_surface={}", xdg_surface_id, wl_surface_id);
        }
        xdg_shell::xdg_wm_base_request::DESTROY => {}
        xdg_shell::xdg_wm_base_request::CREATE_POSITIONER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let pos_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(pos_id, "xdg_positioner", 6, ctx.client_id);
            }
            ctx.shell.set_positioner(pos_id, crate::compositor::shell::PositionerState::default());
        }
        xdg_shell::xdg_wm_base_request::PONG => {}
        _ => {}
    }
}

pub fn handle_surface(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_shell::xdg_surface_request::GET_TOPLEVEL => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let toplevel_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(toplevel_id, "xdg_toplevel", 6, ctx.client_id);
            }
            let wl_surface_id = ctx.xdg_surface_to_wl_surface.get(&ctx.msg.sender_id).copied().unwrap_or(ctx.msg.sender_id);
            ctx.shell.create_toplevel(toplevel_id, wl_surface_id);

            println!("[edgerun-compositor] xdg_toplevel created: toplevel_id={} wl_surface={}", toplevel_id, wl_surface_id);

            let old_kb_focus = ctx.seat.set_keyboard_focus(Some(wl_surface_id));
            let _ = old_kb_focus;
            let old_ptr_focus = ctx.seat.set_pointer_focus(Some(wl_surface_id));
            let _ = old_ptr_focus;

            let serial = *ctx.config_serial;
            *ctx.config_serial += 1;
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                let state = ctx.shell.toplevel_state_bytes(toplevel_id);
                client.send_message(xdg_shell::xdg_surface_configure_event(ctx.msg.sender_id, serial));
                client.send_message(xdg_shell::xdg_toplevel_configure_bounds_event(
                    toplevel_id, ctx.shell.output_width, ctx.shell.output_height));
                client.send_message(xdg_shell::xdg_toplevel_configure_event(
                    toplevel_id, ctx.shell.output_width, ctx.shell.output_height, &state));
                let caps: &[u32] = &[1, 2, 3, 4];
                client.send_message(xdg_shell::xdg_toplevel_wm_capabilities_event(toplevel_id, caps));
                let _ = client.flush();
            }

            if let Some(&kb_id) = ctx.client_keyboard_ids.get(&ctx.client_id) {
                let serial = ctx.seat.next_serial();
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(wl_seat::keyboard_enter_event(kb_id, serial, wl_surface_id, &[]));
                }
            }
            if let Some(&ptr_id) = ctx.client_pointer_ids.get(&ctx.client_id) {
                let serial = ctx.seat.next_serial();
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(wl_seat::pointer_enter_event(
                        ptr_id, serial, wl_surface_id, ctx.seat.pointer_x, ctx.seat.pointer_y));
                    client.send_message(wl_seat::pointer_frame_event(ptr_id));
                }
            }
        }
        xdg_shell::xdg_surface_request::GET_POPUP => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let popup_id = cursor_obj.new_id().unwrap_or(0);
            let parent_id = cursor_obj.object().ok();
            let positioner_id = cursor_obj.object().ok();
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(popup_id, "xdg_popup", 6, ctx.client_id);
            }
            let parent_surface = parent_id.and_then(|pid| {
                ctx.shell.toplevels.values().find(|tl| tl.id == pid).map(|tl| tl.surface_id)
            });
            ctx.shell.create_popup(popup_id, ctx.msg.sender_id, parent_surface);

            if let Some(pos_id) = positioner_id {
                if let Some(pos) = ctx.shell.positioners.get(&pos_id) {
                    ctx.shell.set_popup_positioner(
                        popup_id,
                        pos.anchor_rect_x, pos.anchor_rect_y,
                        pos.anchor_rect_width, pos.anchor_rect_height,
                        pos.anchor, pos.gravity,
                        pos.offset_x, pos.offset_y,
                    );
                }
            }
        }
        xdg_shell::xdg_surface_request::ACK_CONFIGURE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let serial = cursor_obj.uint().unwrap_or(0);
            if let Some(tl) = ctx.shell.toplevels.values_mut().find(|tl| tl.surface_id == ctx.msg.sender_id) {
                if tl.configure_serial == Some(serial) {
                    tl.configured = true;
                }
            }
        }
        xdg_shell::xdg_surface_request::SET_WINDOW_GEOMETRY => {
            let _ = &ctx.msg;
        }
        xdg_shell::xdg_surface_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_toplevel(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_shell::xdg_toplevel_request::SET_TITLE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(title)) = cursor_obj.string() {
                if let Some(tl) = ctx.shell.toplevels.values_mut().find(|tl| tl.id == ctx.msg.sender_id) {
                    tl.title = Some(title.0.clone());
                    println!("[edgerun-compositor] Title: {}", title.0);
                }
            }
        }
        xdg_shell::xdg_toplevel_request::SET_APP_ID => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(app_id)) = cursor_obj.string() {
                if let Some(tl) = ctx.shell.toplevels.values_mut().find(|tl| tl.id == ctx.msg.sender_id) {
                    tl.app_id = Some(app_id.0.clone());
                    println!("[edgerun-compositor] App ID: {}", app_id.0);
                }
            }
        }
        xdg_shell::xdg_toplevel_request::SET_MIN_SIZE => { let _ = &ctx.msg; }
        xdg_shell::xdg_toplevel_request::SET_MAX_SIZE => { let _ = &ctx.msg; }
        xdg_shell::xdg_toplevel_request::MINIMIZE | xdg_shell::xdg_toplevel_request::SET_MINIMIZED => {
            let toplevel_id = ctx.shell.toplevels.get(&ctx.msg.sender_id).map(|tl| tl.id);
            if let Some(tl_id) = toplevel_id {
                ctx.shell.toplevels.get_mut(&tl_id).unwrap().minimized = true;
                eprintln!("[edgerun-compositor] Toplevel {} minimized", tl_id);
            }
        }
        xdg_shell::xdg_toplevel_request::SET_FULLSCREEN => {
            let toplevel_id = ctx.shell.toplevels.get(&ctx.msg.sender_id).map(|tl| tl.id);
            if let Some(tl_id) = toplevel_id {
                ctx.shell.toplevels.get_mut(&tl_id).unwrap().minimized = false;
                ctx.shell.set_fullscreen(tl_id, true);
                let surface_id = ctx.shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0);
                if let Some(surface) = ctx.surfaces.get_mut(surface_id) {
                    surface.x = 0;
                    surface.y = 0;
                }
                let serial = ctx.shell.configure_toplevel_with_state(tl_id, ctx.shell.output_width, ctx.shell.output_height);
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    let state = ctx.shell.toplevel_state_bytes(tl_id);
                    client.send_message(xdg_shell::xdg_surface_configure_event(surface_id, serial));
                    client.send_message(xdg_shell::xdg_toplevel_configure_event(
                        tl_id, ctx.shell.output_width, ctx.shell.output_height, &state));
                }
            }
        }
        xdg_shell::xdg_toplevel_request::UNSET_FULLSCREEN => {
            let toplevel_id = ctx.shell.toplevels.get(&ctx.msg.sender_id).map(|tl| tl.id);
            if let Some(tl_id) = toplevel_id {
                ctx.shell.set_fullscreen(tl_id, false);
                let surface_id = ctx.shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0);
                let serial = ctx.shell.configure_toplevel_with_state(tl_id, ctx.shell.output_width, ctx.shell.output_height);
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    let state = ctx.shell.toplevel_state_bytes(tl_id);
                    client.send_message(xdg_shell::xdg_surface_configure_event(surface_id, serial));
                    client.send_message(xdg_shell::xdg_toplevel_configure_event(
                        tl_id, ctx.shell.output_width, ctx.shell.output_height, &state));
                }
            }
        }
        xdg_shell::xdg_toplevel_request::MAXIMIZE | xdg_shell::xdg_toplevel_request::SET_MAXIMIZED => {
            let toplevel_id = ctx.shell.toplevels.get(&ctx.msg.sender_id).map(|tl| tl.id);
            if let Some(tl_id) = toplevel_id {
                ctx.shell.toplevels.get_mut(&tl_id).unwrap().minimized = false;
                ctx.shell.maximize(tl_id);
                let serial = ctx.shell.configure_toplevel_with_state(tl_id, ctx.shell.output_width, ctx.shell.output_height);
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    let state = ctx.shell.toplevel_state_bytes(tl_id);
                    let surface_id = ctx.shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0);
                    client.send_message(xdg_shell::xdg_surface_configure_event(surface_id, serial));
                    client.send_message(xdg_shell::xdg_toplevel_configure_event(
                        tl_id, ctx.shell.output_width, ctx.shell.output_height, &state));
                }
            }
        }
        xdg_shell::xdg_toplevel_request::UNMAXIMIZE | xdg_shell::xdg_toplevel_request::UNSET_MAXIMIZED => {
            let toplevel_id = ctx.shell.toplevels.get(&ctx.msg.sender_id).map(|tl| tl.id);
            if let Some(tl_id) = toplevel_id {
                ctx.shell.unmaximize(tl_id);
                let serial = ctx.shell.configure_toplevel_with_state(tl_id, ctx.shell.output_width, ctx.shell.output_height);
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    let state = ctx.shell.toplevel_state_bytes(tl_id);
                    let surface_id = ctx.shell.toplevels.get(&tl_id).map(|tl| tl.surface_id).unwrap_or(0);
                    client.send_message(xdg_shell::xdg_surface_configure_event(surface_id, serial));
                    client.send_message(xdg_shell::xdg_toplevel_configure_event(
                        tl_id, ctx.shell.output_width, ctx.shell.output_height, &state));
                }
            }
        }
        xdg_shell::xdg_toplevel_request::MOVE => {
            let _ = &ctx.msg;
            // Client initiated interactive move. Track that the window is being moved.
            // Full implementation would start an interactive grab on the pointer.
            if let Some(tl) = ctx.shell.toplevels.get(&ctx.msg.sender_id) {
                eprintln!("[edgerun-compositor] Toplevel {} initiated interactive move", tl.id);
            }
        }
        xdg_shell::xdg_toplevel_request::RESIZE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _seat_obj_id = cursor_obj.object().unwrap_or(0);
            let _serial = cursor_obj.uint().unwrap_or(0);
            let edges = cursor_obj.uint().unwrap_or(0);
            if let Some(tl) = ctx.shell.toplevels.get_mut(&ctx.msg.sender_id) {
                tl.resizing = true;
                eprintln!("[edgerun-compositor] Toplevel {} initiated interactive resize (edges={})", tl.id, edges);
            }
        }
        xdg_shell::xdg_toplevel_request::SET_PARENT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let parent_obj_id = cursor_obj.object().unwrap_or(0);
            let parent_id = if parent_obj_id != 0 {
                ctx.shell.toplevels.values().find(|p| p.id == parent_obj_id).map(|p| p.id)
            } else {
                None
            };
            if let Some(tl) = ctx.shell.toplevels.get_mut(&ctx.msg.sender_id) {
                tl.parent = parent_id;
                eprintln!("[edgerun-compositor] Toplevel {} set parent to {:?}", tl.id, parent_id);
            }
        }
        xdg_shell::xdg_toplevel_request::DESTROY => {
            let surface_id = ctx.shell.toplevels.get(&ctx.msg.sender_id).map(|tl| tl.surface_id);
            if ctx.seat.keyboard_focus() == surface_id {
                let serial = ctx.seat.next_serial();
                if let Some(sid) = surface_id {
                    for (&cid, &kb_id) in ctx.client_keyboard_ids.iter() {
                        if let Some(client) = ctx.server.client_mut(cid) {
                            client.send_message(wl_seat::keyboard_leave_event(kb_id, serial, sid));
                        }
                    }
                }
                ctx.seat.set_keyboard_focus(None);
            }
            if ctx.seat.pointer_focus() == surface_id {
                let serial = ctx.seat.next_serial();
                if let Some(sid) = surface_id {
                    for (&cid, &ptr_id) in ctx.client_pointer_ids.iter() {
                        if let Some(client) = ctx.server.client_mut(cid) {
                            client.send_message(wl_seat::pointer_leave_event(ptr_id, serial, sid));
                        }
                    }
                }
                ctx.seat.set_pointer_focus(None);
            }
            ctx.shell.destroy_toplevel(ctx.msg.sender_id);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                if let Some(sid) = surface_id {
                    reg.destroy(sid);
                }
            }
        }
        _ => {}
    }
}

pub fn handle_positioner(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_shell::xdg_positioner_request::SET_SIZE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) {
                pos.width = w; pos.height = h;
            }
        }
        xdg_shell::xdg_positioner_request::SET_ANCHOR_RECT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) {
                pos.anchor_rect_x = x; pos.anchor_rect_y = y;
                pos.anchor_rect_width = w; pos.anchor_rect_height = h;
            }
        }
        xdg_shell::xdg_positioner_request::SET_ANCHOR => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let anchor = cursor_obj.uint().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) { pos.anchor = anchor; }
        }
        xdg_shell::xdg_positioner_request::SET_GRAVITY => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let gravity = cursor_obj.uint().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) { pos.gravity = gravity; }
        }
        xdg_shell::xdg_positioner_request::SET_CONSTRAINT_ADJUSTMENT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let adjustment = cursor_obj.uint().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) {
                pos.constraint_adjustment = adjustment;
            }
        }
        xdg_shell::xdg_positioner_request::SET_OFFSET => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) {
                pos.offset_x = x; pos.offset_y = y;
            }
        }
        xdg_shell::xdg_positioner_request::SET_REACTIVE => {
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) {
                pos.reactive = true;
            }
        }
        xdg_shell::xdg_positioner_request::SET_PARENT_SIZE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) {
                pos.parent_width = w;
                pos.parent_height = h;
            }
        }
        xdg_shell::xdg_positioner_request::SET_PARENT_CONFIGURE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let serial = cursor_obj.uint().unwrap_or(0);
            if let Some(pos) = ctx.shell.positioners.get_mut(&ctx.msg.sender_id) {
                pos.parent_configure_serial = serial;
            }
        }
        xdg_shell::xdg_positioner_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.shell.positioners.remove(&ctx.msg.sender_id);
        }
        _ => {}
    }
}

pub fn handle_popup(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_shell::xdg_popup_request::DESTROY => {
            ctx.shell.destroy_popup(ctx.msg.sender_id);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        xdg_shell::xdg_popup_request::GRAB => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let seat_obj_id = cursor_obj.object().unwrap_or(0);
            let serial = cursor_obj.uint().unwrap_or(0);
            let _ = &seat_obj_id;
            ctx.shell.grab_popup(ctx.msg.sender_id, seat_obj_id, serial);
            if let Some(popup) = ctx.shell.popups.get(&ctx.msg.sender_id) {
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(xdg_shell::xdg_popup_configure_event(
                        ctx.msg.sender_id,
                        popup.x, popup.y,
                        popup.width.max(100), popup.height.max(50),
                    ));
                    let _ = client.flush();
                }
            }
        }
        xdg_shell::xdg_popup_request::REPOSITION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _positioner_id = cursor_obj.object().unwrap_or(0);
            let _token = cursor_obj.uint().unwrap_or(0);
        }
        _ => {}
    }
}
