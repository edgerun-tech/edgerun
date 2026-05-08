//! Input event processing for Wayland devices.
//! Extracted from the original dispatch module.

use std::collections::HashMap;

use crate::compositor::seat::Seat;
use crate::compositor::shell::Shell;
use crate::compositor::surface::SurfaceTree;
use crate::input::evdev::EvdevManager;
use crate::input::keymap::{self, Keymap, Modifiers};
pub use crate::protocol::dispatch::{ConstraintType, PointerConstraint, TouchSlot, TouchState};
use crate::render::cursor::Cursor;
use crate::server::WaylandServer;
use edgerun_protocols::wayland::wl_seat;
use edgerun_protocols::wayland::zwp_pointer_constraints;
use edgerun_protocols::wayland::zwp_pointer_gestures;
use edgerun_protocols::wayland::zwp_relative_pointer;

const REL_X: u16 = 0;
const REL_Y: u16 = 1;
const ABS_AXIS_MAX: f64 = 32767.0;

fn relative_motion_delta(code: u16, value: i32) -> (f64, f64) {
    match code {
        REL_X => (value as f64, 0.0),
        REL_Y => (0.0, value as f64),
        _ => (0.0, 0.0),
    }
}

fn absolute_axis_to_output(value: i32, extent: i32) -> f64 {
    let normalized = (value as f64 / ABS_AXIS_MAX).clamp(0.0, 1.0);
    normalized * extent.max(0) as f64
}

/// Process input events for a single device (called from epoll event handler).
pub fn process_input_for_device(
    input_mgr: &mut EvdevManager,
    dev_id: u32,
    seat: &mut Seat,
    keymap: &mut Keymap,
    modifiers: &mut Modifiers,
    server: &mut WaylandServer,
    surfaces: &SurfaceTree,
    shell: &mut Shell,
    client_keyboard_ids: &HashMap<u32, u32>,
    client_pointer_ids: &HashMap<u32, u32>,
    client_touch_ids: &HashMap<u32, u32>,
    client_relative_pointer_ids: &HashMap<u32, u32>,
    client_gesture_swipe_ids: &HashMap<u32, u32>,
    client_gesture_pinch_ids: &HashMap<u32, u32>,
    _client_compositor_ids: &HashMap<u32, u32>,
    cursor: &mut Cursor,
    touch_state: &mut TouchState,
    pointer_constraints: &mut HashMap<u32, PointerConstraint>,
    constraint_type_map: &mut HashMap<u32, ConstraintType>,
) {
    for event in input_mgr.read_events(dev_id, 64) {
        match event.kind {
            edgerun_devices::input::InputEventKind::Key => {
                let scancode = event.code as u16;
                let pressed = event.value == 1;

                if event.code >= 0x110 && event.code <= 0x11f {
                    let serial = seat.next_serial();
                    if pressed {
                        let tl_id_and_sid = shell.frontmost().map(|tl| (tl.id, tl.surface_id));
                        let tl_id_and_sid = tl_id_and_sid;
                        if let Some((tl_id, sid)) = tl_id_and_sid {
                            if seat.keyboard_focus() != Some(sid) {
                                if let Some(old_sid) = seat.keyboard_focus() {
                                    for (&cid, &kb_id) in client_keyboard_ids.iter() {
                                        if let Some(client) = server.client_mut(cid) {
                                            client.send_message(wl_seat::keyboard_leave_event(
                                                kb_id, serial, old_sid,
                                            ));
                                        }
                                    }
                                }
                                seat.set_keyboard_focus(Some(sid));
                                for (&cid, &kb_id) in client_keyboard_ids.iter() {
                                    if let Some(client) = server.client_mut(cid) {
                                        client.send_message(wl_seat::keyboard_enter_event(
                                            kb_id,
                                            serial,
                                            sid,
                                            &[],
                                        ));
                                        client.send_message(wl_seat::keyboard_modifiers_event(
                                            kb_id,
                                            serial,
                                            if modifiers.shift { 1 } else { 0 },
                                            if modifiers.caps { 2 } else { 0 },
                                            0,
                                            0,
                                        ));
                                    }
                                }
                                shell.activate(tl_id);
                                let output_w = shell.output_width;
                                let output_h = shell.output_height;
                                let cfg_serial =
                                    shell.configure_toplevel_with_state(tl_id, output_w, output_h);
                                for (&cid, _) in client_keyboard_ids.iter() {
                                    if let Some(client) = server.client_mut(cid) {
                                        let state = shell.toplevel_state_bytes(tl_id);
                                        client.send_message(
                                            edgerun_protocols::wayland::xdg_shell::xdg_surface_configure_event(
                                                sid, cfg_serial,
                                            ),
                                        );
                                        client.send_message(edgerun_protocols::wayland::xdg_shell::xdg_toplevel_configure_event(tl_id, output_w, output_h, &state));
                                    }
                                }
                            }
                        }
                    }
                    for (&cid, &ptr_id) in client_pointer_ids.iter() {
                        if let Some(client) = server.client_mut(cid) {
                            client.send_message(wl_seat::pointer_button_event(
                                ptr_id,
                                serial,
                                event.timestamp_sec as u32,
                                event.code as u32,
                                if pressed {
                                    wl_seat::button_state::PRESSED
                                } else {
                                    wl_seat::button_state::RELEASED
                                },
                            ));
                            client.send_message(wl_seat::pointer_frame_event(ptr_id));
                        }
                    }
                    continue;
                }

                keymap::process_key_event(scancode, pressed, modifiers);
                let keysym = keymap.translate(scancode, modifiers);
                let serial = seat.next_serial();
                for (&client_id, &kb_id) in client_keyboard_ids {
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(wl_seat::keyboard_key_event(
                            kb_id,
                            serial,
                            event.timestamp_sec as u32,
                            scancode as u32,
                            if pressed {
                                wl_seat::key_state::PRESSED
                            } else {
                                wl_seat::key_state::RELEASED
                            },
                        ));
                        client.send_message(wl_seat::keyboard_modifiers_event(
                            kb_id,
                            serial,
                            if modifiers.shift { 1 } else { 0 },
                            if modifiers.caps { 2 } else { 0 },
                            0,
                            0,
                        ));
                    }
                }
                if pressed
                    && modifiers.shift
                    && matches!(keysym, keymap::Keysym::Special(keymap::SpecialKey::Escape))
                {
                    println!("[edgerun-compositor] Shift+ESC pressed, exiting...");
                    std::process::exit(0);
                }
            }
            edgerun_devices::input::InputEventKind::RelativeMotion => {
                let (dx, dy) = relative_motion_delta(event.code, event.value);
                seat.pointer_x += dx;
                seat.pointer_y += dy;

                let mut is_locked = false;
                let mut locked_constraint_id = None;
                for (constraint_id, constraint) in pointer_constraints.iter_mut() {
                    if constraint.surface_id == seat.pointer_focus().unwrap_or(0)
                        && constraint.activated
                    {
                        match constraint_type_map.get(constraint_id) {
                            Some(ConstraintType::Lock) => {
                                is_locked = true;
                                locked_constraint_id = Some(*constraint_id);
                            }
                            Some(ConstraintType::Confine) => {
                                if let Some((rx, ry, rw, rh)) = constraint.region {
                                    seat.pointer_x =
                                        seat.pointer_x.max(rx as f64).min((rx + rw) as f64);
                                    seat.pointer_y =
                                        seat.pointer_y.max(ry as f64).min((ry + rh) as f64);
                                }
                            }
                            None => {}
                        }
                    }
                }
                if !is_locked {
                    cursor.x = seat.pointer_x as i32;
                    cursor.y = seat.pointer_y as i32;
                }
                let pointer_hit = if is_locked {
                    None
                } else {
                    shell.surface_at(surfaces, seat.pointer_x as i32, seat.pointer_y as i32)
                };
                let pointer_focus = pointer_hit.map(|hit| hit.surface_id);
                let old_pointer_focus = seat.pointer_focus();
                let serial = seat.next_serial();
                if !is_locked && seat.set_pointer_focus(pointer_focus) {
                    for (&client_id, &ptr_id) in client_pointer_ids {
                        if let Some(client) = server.client_mut(client_id) {
                            if let Some(old_surface_id) = old_pointer_focus {
                                client.send_message(wl_seat::pointer_leave_event(
                                    ptr_id,
                                    serial,
                                    old_surface_id,
                                ));
                            }
                            if let Some(hit) = pointer_hit {
                                let (surface_x, surface_y) =
                                    hit.local_position(seat.pointer_x, seat.pointer_y);
                                client.send_message(wl_seat::pointer_enter_event(
                                    ptr_id,
                                    serial,
                                    hit.surface_id,
                                    surface_x,
                                    surface_y,
                                ));
                            }
                            client.send_message(wl_seat::pointer_frame_event(ptr_id));
                        }
                    }
                }
                for (&client_id, &ptr_id) in client_pointer_ids {
                    if let Some(client) = server.client_mut(client_id) {
                        if is_locked {
                            if let Some(cid) = locked_constraint_id {
                                client.send_message(
                                    zwp_pointer_constraints::locked_pointer_motion_event(
                                        cid,
                                        event.timestamp_sec as u32,
                                        (dx * 65536.0) as i32 as u32,
                                        (dy * 65536.0) as i32 as u32,
                                    ),
                                );
                            }
                        } else if let Some(hit) = pointer_hit {
                            let (surface_x, surface_y) =
                                hit.local_position(seat.pointer_x, seat.pointer_y);
                            client.send_message(wl_seat::pointer_motion_event(
                                ptr_id,
                                event.timestamp_sec as u32,
                                surface_x,
                                surface_y,
                            ));
                        }
                        client.send_message(wl_seat::pointer_frame_event(ptr_id));
                    }
                }
                let dx_fixed = (dx * 65536.0) as i32 as u32;
                let dy_fixed = (dy * 65536.0) as i32 as u32;
                let utime = event.timestamp_sec as u64;
                for (&client_id, &rel_ptr_id) in client_relative_pointer_ids {
                    if let Some(client) = server.client_mut(client_id) {
                        client.send_message(zwp_relative_pointer::relative_motion_event(
                            rel_ptr_id,
                            (utime >> 32) as u32,
                            utime as u32,
                            dx_fixed,
                            dy_fixed,
                            dx_fixed,
                            dy_fixed,
                        ));
                    }
                }
            }
            edgerun_devices::input::InputEventKind::AbsoluteMotion => {
                const ABS_MT_SLOT: u16 = 0x3f;
                const ABS_MT_TRACKING_ID: u16 = 0x39;
                const ABS_MT_POSITION_X: u16 = 0x35;
                const ABS_MT_POSITION_Y: u16 = 0x36;
                const ABS_X: u16 = 0x00;
                const ABS_Y: u16 = 0x01;

                match event.code {
                    ABS_MT_SLOT => {
                        touch_state.current_slot = event.value;
                        touch_state
                            .slots
                            .entry(event.value)
                            .or_insert_with(|| TouchSlot {
                                touch_id: touch_state.next_touch_id,
                                surface_id: None,
                                client_id: None,
                                origin_x: 0,
                                origin_y: 0,
                                x: 0.0,
                                y: 0.0,
                                active: false,
                                has_x: false,
                                has_y: false,
                                down_sent: false,
                            });
                    }
                    ABS_MT_TRACKING_ID => {
                        let slot = touch_state.current_slot;
                        if event.value < 0 {
                            if let Some(touch_slot) = touch_state.slots.get_mut(&slot) {
                                if touch_slot.active
                                    && touch_slot.down_sent
                                    && touch_slot.surface_id.is_some()
                                {
                                    let serial = seat.next_serial();
                                    for (&cid, &touch_obj_id) in client_touch_ids.iter() {
                                        if let Some(client) = server.client_mut(cid) {
                                            client.send_message(wl_seat::touch_up_event(
                                                touch_obj_id,
                                                serial,
                                                event.timestamp_sec as u32,
                                                slot,
                                            ));
                                            client.send_message(wl_seat::touch_frame_event(
                                                touch_obj_id,
                                            ));
                                        }
                                    }
                                    if seat.touch_focus() == touch_slot.surface_id {
                                        seat.set_touch_focus(None);
                                    }
                                }
                                touch_slot.active = false;
                                touch_slot.surface_id = None;
                                touch_slot.down_sent = false;
                                touch_slot.has_x = false;
                                touch_slot.has_y = false;
                            }
                        } else {
                            let slot_entry =
                                touch_state.slots.entry(slot).or_insert_with(|| TouchSlot {
                                    touch_id: touch_state.next_touch_id,
                                    surface_id: None,
                                    client_id: None,
                                    origin_x: 0,
                                    origin_y: 0,
                                    x: 0.0,
                                    y: 0.0,
                                    active: false,
                                    has_x: false,
                                    has_y: false,
                                    down_sent: false,
                                });
                            slot_entry.touch_id = event.value as u32;
                            slot_entry.active = true;
                            slot_entry.surface_id = None;
                            slot_entry.down_sent = false;
                            slot_entry.has_x = false;
                            slot_entry.has_y = false;
                            touch_state.next_touch_id += 1;
                        }
                    }
                    ABS_MT_POSITION_X => {
                        let slot = touch_state.current_slot;
                        if let Some(touch_slot) = touch_state.slots.get_mut(&slot) {
                            touch_slot.x = absolute_axis_to_output(event.value, shell.output_width);
                            touch_slot.has_x = true;
                            send_touch_slot_update(
                                touch_slot,
                                slot,
                                event.timestamp_sec as u32,
                                seat,
                                server,
                                surfaces,
                                shell,
                                client_touch_ids,
                            );
                        }

                        detect_and_send_gestures(
                            touch_state,
                            seat,
                            server,
                            client_gesture_swipe_ids,
                            client_gesture_pinch_ids,
                            event.timestamp_sec as u32,
                        );
                    }
                    ABS_MT_POSITION_Y => {
                        let slot = touch_state.current_slot;
                        if let Some(touch_slot) = touch_state.slots.get_mut(&slot) {
                            touch_slot.y =
                                absolute_axis_to_output(event.value, shell.output_height);
                            touch_slot.has_y = true;
                            send_touch_slot_update(
                                touch_slot,
                                slot,
                                event.timestamp_sec as u32,
                                seat,
                                server,
                                surfaces,
                                shell,
                                client_touch_ids,
                            );
                        }
                        detect_and_send_gestures(
                            touch_state,
                            seat,
                            server,
                            client_gesture_swipe_ids,
                            client_gesture_pinch_ids,
                            event.timestamp_sec as u32,
                        );
                    }
                    ABS_X | ABS_Y => {
                        let slot = 0;
                        touch_state.current_slot = slot;
                        let touch_slot =
                            touch_state.slots.entry(slot).or_insert_with(|| TouchSlot {
                                touch_id: 0,
                                surface_id: None,
                                client_id: None,
                                origin_x: 0,
                                origin_y: 0,
                                x: 0.0,
                                y: 0.0,
                                active: true,
                                has_x: false,
                                has_y: false,
                                down_sent: false,
                            });
                        if event.code == ABS_X {
                            touch_slot.x = absolute_axis_to_output(event.value, shell.output_width);
                            touch_slot.has_x = true;
                        } else {
                            touch_slot.y =
                                absolute_axis_to_output(event.value, shell.output_height);
                            touch_slot.has_y = true;
                        }
                        send_touch_slot_update(
                            touch_slot,
                            slot,
                            event.timestamp_sec as u32,
                            seat,
                            server,
                            surfaces,
                            shell,
                            client_touch_ids,
                        );
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn send_touch_slot_update(
    touch_slot: &mut TouchSlot,
    slot: i32,
    time: u32,
    seat: &mut Seat,
    server: &mut WaylandServer,
    surfaces: &SurfaceTree,
    shell: &Shell,
    client_touch_ids: &HashMap<u32, u32>,
) {
    if !touch_slot.active || !touch_slot.has_x || !touch_slot.has_y {
        return;
    }

    if !touch_slot.down_sent {
        let Some(hit) = shell.surface_at(surfaces, touch_slot.x as i32, touch_slot.y as i32) else {
            return;
        };
        let (surface_x, surface_y) = hit.local_position(touch_slot.x, touch_slot.y);
        touch_slot.surface_id = Some(hit.surface_id);
        touch_slot.origin_x = hit.x;
        touch_slot.origin_y = hit.y;
        touch_slot.down_sent = true;
        seat.set_touch_focus(Some(hit.surface_id));

        let serial = seat.next_serial();
        for (&cid, &touch_obj_id) in client_touch_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(wl_seat::touch_down_event(
                    touch_obj_id,
                    serial,
                    time,
                    hit.surface_id,
                    slot,
                    surface_x,
                    surface_y,
                ));
                client.send_message(wl_seat::touch_frame_event(touch_obj_id));
            }
        }
        return;
    }

    let Some(_) = touch_slot.surface_id else {
        return;
    };
    let surface_x = touch_slot.x - touch_slot.origin_x as f64;
    let surface_y = touch_slot.y - touch_slot.origin_y as f64;
    for (&cid, &touch_obj_id) in client_touch_ids.iter() {
        if let Some(client) = server.client_mut(cid) {
            client.send_message(wl_seat::touch_motion_event(
                touch_obj_id,
                time,
                slot,
                surface_x,
                surface_y,
            ));
            client.send_message(wl_seat::touch_frame_event(touch_obj_id));
        }
    }
}

fn detect_and_send_gestures(
    touch_state: &mut TouchState,
    seat: &mut Seat,
    server: &mut WaylandServer,
    client_gesture_swipe_ids: &HashMap<u32, u32>,
    client_gesture_pinch_ids: &HashMap<u32, u32>,
    time: u32,
) {
    let active_slots: Vec<_> = touch_state
        .slots
        .iter()
        .filter(|(_, s)| s.active)
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    let finger_count = active_slots.len() as u32;

    if finger_count < 2 {
        if touch_state.gesture_in_progress {
            if touch_state.swipe_active {
                let serial = seat.next_serial();
                for (&_cid, &swipe_id) in client_gesture_swipe_ids.iter() {
                    if let Some(client) = server.client_mut(_cid) {
                        client.send_message(zwp_pointer_gestures::swipe_end_event(
                            swipe_id, serial, time, 0,
                        ));
                    }
                }
                touch_state.swipe_active = false;
            }
            if touch_state.pinch_active {
                let serial = seat.next_serial();
                for (&_cid, &pinch_id) in client_gesture_pinch_ids.iter() {
                    if let Some(client) = server.client_mut(_cid) {
                        client.send_message(zwp_pointer_gestures::pinch_end_event(
                            pinch_id, serial, time, 0,
                        ));
                    }
                }
                touch_state.pinch_active = false;
            }
            touch_state.gesture_in_progress = false;
        }
        touch_state.active_fingers = finger_count;
        return;
    }

    let sum_x: f64 = active_slots.iter().map(|(_, s)| s.x).sum();
    let sum_y: f64 = active_slots.iter().map(|(_, s)| s.y).sum();
    let centroid_x = sum_x / finger_count as f64;
    let centroid_y = sum_y / finger_count as f64;

    let mut total_dist = 0.0;
    let mut pair_count = 0;
    for (i, (_, s1)) in active_slots.iter().enumerate() {
        for (_, s2) in active_slots.iter().skip(i + 1) {
            let dx = s1.x - s2.x;
            let dy = s1.y - s2.y;
            total_dist += (dx * dx + dy * dy).sqrt();
            pair_count += 1;
        }
    }
    let avg_dist = if pair_count > 0 {
        total_dist / pair_count as f64
    } else {
        0.0
    };

    if !touch_state.gesture_in_progress {
        touch_state.gesture_in_progress = true;
        touch_state.gesture_finger_count = finger_count;
        touch_state.gesture_start_time = time;
        touch_state.initial_pinch_distance = avg_dist;
        touch_state.prev_centroid_x = centroid_x;
        touch_state.prev_centroid_y = centroid_y;
        touch_state.swipe_active = true;
        touch_state.pinch_active = true;
        let serial = seat.next_serial();
        let surface_id = active_slots
            .first()
            .map(|(_, s)| s.surface_id)
            .flatten()
            .unwrap_or(0);
        for (&cid, &swipe_id) in client_gesture_swipe_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::swipe_begin_event(
                    swipe_id,
                    serial,
                    time,
                    surface_id,
                    finger_count,
                ));
            }
        }
        for (&cid, &pinch_id) in client_gesture_pinch_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::pinch_begin_event(
                    pinch_id,
                    serial,
                    time,
                    surface_id,
                    finger_count,
                ));
            }
        }
    }

    let dx = centroid_x - touch_state.prev_centroid_x;
    let dy = centroid_y - touch_state.prev_centroid_y;

    if touch_state.swipe_active && (dx.abs() > 0.0001 || dy.abs() > 0.0001) {
        let dx_fixed = (dx * 65536.0 * 10.0) as i32 as u32;
        let dy_fixed = (dy * 65536.0 * 10.0) as i32 as u32;
        for (&cid, &swipe_id) in client_gesture_swipe_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::swipe_update_event(
                    swipe_id, time, dx_fixed, dy_fixed,
                ));
            }
        }
    }

    if touch_state.pinch_active && touch_state.initial_pinch_distance > 0.0 {
        let scale = if touch_state.initial_pinch_distance > 0.0001 {
            (avg_dist / touch_state.initial_pinch_distance * 65536.0) as u32
        } else {
            65536
        };
        let rotation = 0u32;
        let dx_fixed = (dx * 65536.0 * 10.0) as i32 as u32;
        let dy_fixed = (dy * 65536.0 * 10.0) as i32 as u32;
        for (&cid, &pinch_id) in client_gesture_pinch_ids.iter() {
            if let Some(client) = server.client_mut(cid) {
                client.send_message(zwp_pointer_gestures::pinch_update_event(
                    pinch_id, time, dx_fixed, dy_fixed, scale, rotation,
                ));
            }
        }
    }

    touch_state.prev_centroid_x = centroid_x;
    touch_state.prev_centroid_y = centroid_y;
    touch_state.active_fingers = finger_count;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_motion_delta_preserves_axis() {
        assert_eq!(relative_motion_delta(REL_X, 7), (7.0, 0.0));
        assert_eq!(relative_motion_delta(REL_Y, -3), (0.0, -3.0));
        assert_eq!(relative_motion_delta(99, 4), (0.0, 0.0));
    }

    #[test]
    fn absolute_axis_to_output_clamps_to_extent() {
        assert_eq!(absolute_axis_to_output(0, 200), 0.0);
        assert_eq!(absolute_axis_to_output(32767, 200), 200.0);
        assert_eq!(absolute_axis_to_output(-10, 200), 0.0);
        assert_eq!(absolute_axis_to_output(40000, 200), 200.0);
    }
}
