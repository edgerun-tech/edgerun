//! xdg decoration, activation, output handlers.

use super::DispatchContext;
use crate::protocol::xdg_decoration;
use crate::protocol::xdg_activation;
use crate::protocol::xdg_output;
use crate::protocol::xdg_shell;
use crate::protocol::wl_seat;
use crate::wire::decode::ArgCursor;

pub fn handle_decoration_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_decoration::decoration_manager_request::GET_TOPLEVEL_DECORATION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let decoration_id = cursor_obj.new_id().unwrap_or(0);
            let toplevel_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(decoration_id, "zxdg_toplevel_decoration_v1", 1, ctx.client_id);
            }
            ctx.client_decoration_ids.insert(ctx.client_id, decoration_id);
            if let Some(tl) = ctx.shell.toplevels.values_mut().find(|tl| tl.id == toplevel_id) {
                tl.decoration_id = Some(decoration_id);
                tl.decoration_mode = Some(xdg_decoration::decoration_mode::SERVER_SIDE);
            }
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(xdg_decoration::toplevel_decoration_configure_event(
                    decoration_id, xdg_decoration::decoration_mode::SERVER_SIDE));
            }
        }
        xdg_decoration::decoration_manager_request::DESTROY => {}
        _ => {}
    }
}

pub fn handle_toplevel_decoration(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_decoration::toplevel_decoration_request::SET_MODE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let mode = cursor_obj.uint().unwrap_or(0);
            if let Some(tl) = ctx.shell.toplevels.values_mut().find(|tl| tl.decoration_id == Some(ctx.msg.sender_id)) {
                tl.decoration_mode = Some(mode);
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(xdg_decoration::toplevel_decoration_configure_event(ctx.msg.sender_id, mode));
                }
            }
        }
        xdg_decoration::toplevel_decoration_request::UNSET_MODE => {
            if let Some(tl) = ctx.shell.toplevels.values_mut().find(|tl| tl.decoration_id == Some(ctx.msg.sender_id)) {
                tl.decoration_mode = None;
            }
        }
        xdg_decoration::toplevel_decoration_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_decoration_ids.remove(&ctx.client_id);
        }
        _ => {}
    }
}

pub fn handle_activation(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_activation::xdg_activation_request::GET_ACTIVATION_TOKEN => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let token_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(token_id, "xdg_activation_token_v1", 1, ctx.client_id);
            }
        }
        xdg_activation::xdg_activation_request::ACTIVATE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _token = cursor_obj.string().ok().flatten();
            let surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(tl) = ctx.shell.toplevel_for_surface(surface_id) {
                let tl_id = tl.id;
                ctx.shell.activate(tl_id);
                let serial = *ctx.config_serial;
                *ctx.config_serial += 1;
                if let Some(surface) = ctx.surfaces.get_mut(surface_id) {
                    if ctx.seat.keyboard_focus() != Some(surface_id) {
                        if let Some(old_sid) = ctx.seat.keyboard_focus() {
                            for (&cid, &kb_id) in ctx.client_keyboard_ids.iter() {
                                if let Some(client) = ctx.server.client_mut(cid) {
                                    client.send_message(wl_seat::keyboard_leave_event(kb_id, serial, old_sid));
                                }
                            }
                        }
                        ctx.seat.set_keyboard_focus(Some(surface_id));
                        for (&cid, &kb_id) in ctx.client_keyboard_ids.iter() {
                            if let Some(client) = ctx.server.client_mut(cid) {
                                client.send_message(wl_seat::keyboard_enter_event(kb_id, serial, surface_id, &[]));
                                client.send_message(wl_seat::keyboard_modifiers_event(
                                    kb_id, serial,
                                    if ctx.modifiers.shift { 1 } else { 0 },
                                    if ctx.modifiers.caps { 2 } else { 0 },
                                    0, 0,
                                ));
                            }
                        }
                        let output_w = ctx.shell.output_width;
                        let output_h = ctx.shell.output_height;
                        let cfg_serial = ctx.shell.configure_toplevel_with_state(tl_id, output_w, output_h);
                        if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                            let state = ctx.shell.toplevel_state_bytes(tl_id).to_vec();
                            client.send_message(xdg_shell::xdg_toplevel_configure_event(
                                tl_id, output_w, output_h, &state));
                        }
                        eprintln!("[edgerun-compositor] Activated surface {} (toplevel {})", surface_id, tl_id);
                    }
                }
            }
        }
        xdg_activation::xdg_activation_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_activation_token(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_activation::xdg_activation_token_request::SET_SERIAL => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let serial = cursor_obj.uint().unwrap_or(0);
            let _surface_id = cursor_obj.object().unwrap_or(0);
            // Store serial for token generation
            ctx.shell.set_pending_token_serial(ctx.msg.sender_id, serial as u64);
        }
        xdg_activation::xdg_activation_token_request::SET_APP_ID => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(app_id)) = cursor_obj.string() {
                ctx.shell.set_pending_token_app_id(ctx.msg.sender_id, &app_id.0);
            }
        }
        xdg_activation::xdg_activation_token_request::SET_SURFACE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_id = cursor_obj.object().unwrap_or(0);
            ctx.shell.set_pending_token_surface(ctx.msg.sender_id, surface_id);
        }
        xdg_activation::xdg_activation_token_request::COMMIT => {
            // Generate activation token and send done event
            let token = format!("{:x}-{:x}", ctx.client_id, ctx.msg.sender_id);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(xdg_activation::activation_done_event(ctx.msg.sender_id, &token));
            }
            // Track the token for later activation validation
            ctx.shell.register_activation_token(&token, ctx.client_id);
        }
        xdg_activation::xdg_activation_token_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.shell.pending_token_serial.remove(&ctx.msg.sender_id);
            ctx.shell.pending_token_app_id.remove(&ctx.msg.sender_id);
            ctx.shell.pending_token_surface.remove(&ctx.msg.sender_id);
        }
        _ => {}
    }
}

pub fn handle_output_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_output::output_manager_request::GET_XDG_OUTPUT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let xdg_output_id = cursor_obj.new_id().unwrap_or(0);
            let _wl_output_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(xdg_output_id, xdg_output::ZXDG_OUTPUT_V1, 3, ctx.client_id);
            }
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(xdg_output::xdg_output_logical_position(xdg_output_id, 0, 0));
                client.send_message(xdg_output::xdg_output_logical_size(
                    xdg_output_id, ctx.shell.output_width, ctx.shell.output_height));
                client.send_message(xdg_output::xdg_output_name_event(xdg_output_id, "edgerun-output-0"));
                client.send_message(xdg_output::xdg_output_description_event(
                    xdg_output_id, "edgerun compositor output"));
                client.send_message(xdg_output::xdg_output_done_event(xdg_output_id));
                let _ = client.flush();
            }
        }
        xdg_output::output_manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_output(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_output::output_manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}
