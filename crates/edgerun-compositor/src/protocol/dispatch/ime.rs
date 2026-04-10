//! text_input_v3 and input_method_v2 handlers.

use super::DispatchContext;
use crate::protocol::text_input_v3;
use crate::protocol::text_input_v3::TextInputState;
use crate::protocol::input_method_v2;
use crate::protocol::input_method_v2::IMEState;
use crate::wire::decode::ArgCursor;
use crate::input::keymap;

pub fn handle_text_input_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        text_input_v3::manager_request::GET_TEXT_INPUT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let text_input_id = cursor_obj.new_id().unwrap_or(0);
            let _seat_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(text_input_id, text_input_v3::ZWP_TEXT_INPUT_V3, 1, ctx.client_id);
            }
            *ctx.text_input_state = Some(TextInputState::new(text_input_id, ctx.client_id));
        }
        text_input_v3::manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_text_input(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        text_input_v3::text_input_request::ENABLE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_id = cursor_obj.object().unwrap_or(0);
            let serial = cursor_obj.uint().unwrap_or(0);
            *ctx.text_input_serial = serial;
            if let Some(ti) = ctx.text_input_state.as_mut() {
                ti.enabled = true;
                ti.focused_surface = Some(surface_id);
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(text_input_v3::text_input_enter_event(ti.text_input_id, surface_id));
                    client.send_message(text_input_v3::text_input_done_event(ti.text_input_id, serial));
                    let _ = client.flush();
                }
                if let Some(ime) = &ctx.ime_state {
                    if let Some(ime_client) = ctx.server.client_mut(ime.client_id) {
                        ime_client.send_message(input_method_v2::input_method_activate_event(
                            ime.input_method_id, ti.text_input_id));
                        ime_client.send_message(input_method_v2::input_method_done_event(ime.input_method_id));
                    }
                }
            }
        }
        text_input_v3::text_input_request::DISABLE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_id = cursor_obj.object().unwrap_or(0);
            let serial = cursor_obj.uint().unwrap_or(0);
            *ctx.text_input_serial = serial;
            if let Some(ti) = ctx.text_input_state.as_mut() {
                ti.enabled = false;
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(text_input_v3::text_input_leave_event(ti.text_input_id, surface_id, serial));
                    client.send_message(text_input_v3::text_input_done_event(ti.text_input_id, serial));
                    let _ = client.flush();
                }
                if let Some(ime) = &ctx.ime_state {
                    if let Some(ime_client) = ctx.server.client_mut(ime.client_id) {
                        ime_client.send_message(input_method_v2::input_method_deactivate_event(
                            ime.input_method_id, ti.text_input_id));
                        ime_client.send_message(input_method_v2::input_method_done_event(ime.input_method_id));
                    }
                }
            }
        }
        text_input_v3::text_input_request::SET_SURROUNDING_TEXT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(text)) = cursor_obj.string() {
                let cursor = cursor_obj.uint().unwrap_or(0);
                let anchor = cursor_obj.uint().unwrap_or(0);
                if let Some(ti) = ctx.text_input_state.as_mut() {
                    ti.surrounding_text = text.0;
                    ti.cursor = cursor;
                    ti.anchor = anchor;
                    if let Some(ime) = &ctx.ime_state {
                        if let Some(ime_client) = ctx.server.client_mut(ime.client_id) {
                            ime_client.send_message(input_method_v2::input_method_surround_text_event(
                                ime.input_method_id, &ti.surrounding_text, ti.cursor, ti.anchor));
                            ime_client.send_message(input_method_v2::input_method_done_event(ime.input_method_id));
                        }
                    }
                }
            }
        }
        text_input_v3::text_input_request::SET_TEXT_CHANGE_CAUSE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let cause = cursor_obj.uint().unwrap_or(0);
            if let Some(ti) = ctx.text_input_state.as_mut() {
                ti.text_change_cause = cause;
                if let Some(ime) = &ctx.ime_state {
                    if let Some(ime_client) = ctx.server.client_mut(ime.client_id) {
                        ime_client.send_message(input_method_v2::input_method_text_change_cause_event(
                            ime.input_method_id, cause));
                    }
                }
            }
        }
        text_input_v3::text_input_request::COMMIT => {}
        text_input_v3::text_input_request::GET_SURROUNDING_TEXT => {}
        text_input_v3::text_input_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            *ctx.text_input_state = None;
        }
        _ => {}
    }
}

pub fn handle_input_method_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        input_method_v2::manager_request::GET_INPUT_METHOD => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let im_id = cursor_obj.new_id().unwrap_or(0);
            let _seat_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(im_id, input_method_v2::ZWP_INPUT_METHOD_V2, 1, ctx.client_id);
            }
            *ctx.ime_state = Some(IMEState::new(im_id, ctx.client_id));
            eprintln!("[edgerun-compositor] IME server connected, client={}", ctx.client_id);
        }
        input_method_v2::manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_input_method(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        input_method_v2::input_method_request::COMMIT_STRING => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(text)) = cursor_obj.string() {
                if let Some(ti) = ctx.text_input_state.as_ref() {
                    if let Some(client) = ctx.server.client_mut(ti.client_id) {
                        client.send_message(text_input_v3::text_input_commit_string_event(ti.text_input_id, &text.0));
                        client.send_message(text_input_v3::text_input_done_event(ti.text_input_id, *ctx.text_input_serial));
                        let _ = client.flush();
                    }
                }
            }
        }
        input_method_v2::input_method_request::COMMIT_PREEDIT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(text)) = cursor_obj.string() {
                let cursor_begin = cursor_obj.int().unwrap_or(0);
                let cursor_end = cursor_obj.int().unwrap_or(0);
                if let Some(ti) = ctx.text_input_state.as_ref() {
                    if let Some(client) = ctx.server.client_mut(ti.client_id) {
                        client.send_message(text_input_v3::text_input_preedit_string_event(
                            ti.text_input_id, &text.0, cursor_begin, cursor_end));
                    }
                }
            }
        }
        input_method_v2::input_method_request::DELETE_SURROUNDING_TEXT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let before = cursor_obj.int().unwrap_or(0);
            let after = cursor_obj.int().unwrap_or(0);
            if let Some(ti) = ctx.text_input_state.as_ref() {
                if let Some(client) = ctx.server.client_mut(ti.client_id) {
                    client.send_message(text_input_v3::text_input_delete_surrounding_text_event(
                        ti.text_input_id, before, after));
                }
            }
        }
        input_method_v2::input_method_request::COMMIT => {
            if let Some(ti) = ctx.text_input_state.as_ref() {
                if let Some(client) = ctx.server.client_mut(ti.client_id) {
                    client.send_message(text_input_v3::text_input_done_event(ti.text_input_id, *ctx.text_input_serial));
                    let _ = client.flush();
                }
            }
        }
        input_method_v2::input_method_request::SET_SURROUNDING_TEXT |
        input_method_v2::input_method_request::SET_TEXT_CHANGE_CAUSE |
        input_method_v2::input_method_request::SET_CONTENT_TYPE |
        input_method_v2::input_method_request::AVAILABLE => {
            let _ = ctx.msg;
        }
        input_method_v2::input_method_request::GRAB_KEYBOARD => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let grab_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(grab_id, input_method_v2::ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2, 1, ctx.client_id);
            }
            if let Some(ime) = ctx.ime_state.as_mut() {
                ime.keyboard_grab_id = Some(grab_id);
                ime.keyboard_grab_active = true;
                let keymap_str = keymap::xkb_keymap_text();
                let keymap_size = keymap_str.len() as u32;
                let fd = unsafe {
                    libc::open(b"/dev/shm/edgerun-im-km\0".as_ptr() as *const libc::c_char,
                        libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o600)
                };
                if fd >= 0 {
                    unsafe { libc::unlink(b"/dev/shm/edgerun-im-km\0".as_ptr() as *const libc::c_char) };
                    let written = unsafe { libc::write(fd, keymap_str.as_ptr() as *const _, keymap_str.len()) };
                    if written >= 0 {
                        let dup_fd = unsafe { libc::dup(fd) };
                        if dup_fd >= 0 {
                            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                                client.send_message(input_method_v2::keyboard_grab_keymap_event(
                                    grab_id, 1, dup_fd, keymap_size));
                                client.send_message(input_method_v2::keyboard_grab_repeat_info_event(
                                    grab_id, 25, 300));
                                let _ = client.flush();
                            }
                        }
                    }
                    unsafe { libc::close(fd) };
                }
            }
        }
        input_method_v2::input_method_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            *ctx.ime_state = None;
        }
        _ => {}
    }
}

pub fn handle_keyboard_grab(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        input_method_v2::keyboard_grab_request::DESTROY |
        input_method_v2::keyboard_grab_request::RELEASE => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            if let Some(ime) = ctx.ime_state.as_mut() {
                ime.keyboard_grab_active = false;
                ime.keyboard_grab_id = None;
            }
        }
        _ => {}
    }
}
