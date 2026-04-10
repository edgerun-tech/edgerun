//! wl_seat, wl_keyboard, wl_pointer, wl_touch handlers.

use super::DispatchContext;
use crate::protocol::wl_seat;
use crate::wire::decode::ArgCursor;
use crate::input::keymap;

pub fn handle_seat(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_seat::seat_request::GET_KEYBOARD => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let kb_id = cursor_obj.new_id().unwrap_or(0);
            let kb_version = if let Some(res) = ctx.client_registries.get(&ctx.client_id).and_then(|r| r.get(ctx.msg.sender_id)) {
                res.version.min(7)
            } else {
                7
            };
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(kb_id, "wl_keyboard", kb_version, ctx.client_id);
            }
            ctx.client_keyboard_ids.insert(ctx.client_id, kb_id);
            eprintln!("[edgerun-compositor] Client {} created wl_keyboard id={} (v{})", ctx.client_id, kb_id, kb_version);

            let keymap_str = keymap::xkb_keymap_text();
            let keymap_size = keymap_str.len() as u32;
            let fd = unsafe {
                libc::open(b"/dev/shm/edgerun-km\0".as_ptr() as *const libc::c_char,
                    libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o600)
            };
            if fd < 0 {
                eprintln!("[edgerun-compositor] Failed to create keymap tmpfile");
            } else {
                unsafe { libc::unlink(b"/dev/shm/edgerun-km\0".as_ptr() as *const libc::c_char) };
                let written = unsafe { libc::write(fd, keymap_str.as_ptr() as *const _, keymap_str.len()) };
                if written < 0 {
                    eprintln!("[edgerun-compositor] Failed to write keymap");
                } else {
                    let dup_fd = unsafe { libc::dup(fd) };
                    if dup_fd >= 0 {
                        if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                            client.send_message(wl_seat::keyboard_keymap_event(kb_id, 1, dup_fd, keymap_size));
                            client.send_message(wl_seat::keyboard_repeat_info_event(kb_id, 25, 300));
                            let _ = client.flush();
                        }
                    }
                }
                unsafe { libc::close(fd) };
            }
        }
        wl_seat::seat_request::GET_POINTER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let ptr_id = cursor_obj.new_id().unwrap_or(0);
            let ptr_version = if let Some(res) = ctx.client_registries.get(&ctx.client_id).and_then(|r| r.get(ctx.msg.sender_id)) {
                res.version.min(7)
            } else {
                7
            };
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(ptr_id, "wl_pointer", ptr_version, ctx.client_id);
            }
            ctx.client_pointer_ids.insert(ctx.client_id, ptr_id);
            eprintln!("[edgerun-compositor] Client {} created wl_pointer id={} (v{})", ctx.client_id, ptr_id, ptr_version);
        }
        wl_seat::seat_request::GET_TOUCH => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let touch_id = cursor_obj.new_id().unwrap_or(0);
            let touch_version = if let Some(res) = ctx.client_registries.get(&ctx.client_id).and_then(|r| r.get(ctx.msg.sender_id)) {
                res.version.min(7)
            } else {
                7
            };
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(touch_id, "wl_touch", touch_version, ctx.client_id);
            }
            ctx.client_touch_ids.insert(ctx.client_id, touch_id);
            eprintln!("[edgerun-compositor] Client {} created wl_touch id={} (v{})", ctx.client_id, touch_id, touch_version);
        }
        wl_seat::seat_request::RELEASE => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_keyboard(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_seat::keyboard_request::RELEASE => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_keyboard_ids.remove(&ctx.client_id);
        }
        _ => {}
    }
}

pub fn handle_pointer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_seat::pointer_request::SET_CURSOR => {
            let surface_id = ArgCursor::from_message(&ctx.msg).object().unwrap_or(0);
            ctx.cursor.visible = surface_id != 0;
        }
        wl_seat::pointer_request::RELEASE => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_pointer_ids.remove(&ctx.client_id);
        }
        _ => {}
    }
}

pub fn handle_touch(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_seat::touch_request::RELEASE => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_touch_ids.remove(&ctx.client_id);
        }
        _ => {}
    }
}
