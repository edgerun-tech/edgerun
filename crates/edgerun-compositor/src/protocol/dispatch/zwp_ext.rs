//! zwp_* unstable protocol handlers: pointer_constraints, relative_pointer,
//! pointer_gestures, text_input_v1, idle_inhibit.

use super::{ConstraintType, DispatchContext, PointerConstraint};
use crate::protocol::zwp_pointer_constraints;
use crate::protocol::zwp_pointer_gestures;
use crate::protocol::zwp_relative_pointer;
use crate::protocol::zwp_text_input;
use crate::protocol::zxdg_idle_inhibit;
use crate::wire::decode::ArgCursor;

pub fn handle_pointer_constraints(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_pointer_constraints::constraints_request::LOCK_POINTER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let locked_id = cursor_obj.new_id().unwrap_or(0);
            let surface_id = cursor_obj.object().unwrap_or(0);
            let pointer_id = cursor_obj.object().unwrap_or(0);
            let lifetime = cursor_obj.uint().unwrap_or(0);
            let _region = cursor_obj.object().ok();
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(
                    locked_id,
                    zwp_pointer_constraints::ZWP_LOCKED_POINTER_V1,
                    1,
                    ctx.client_id,
                );
            }
            ctx.pointer_constraints.insert(
                locked_id,
                PointerConstraint {
                    constraint_id: locked_id,
                    surface_id,
                    pointer_id,
                    lifetime,
                    region: None,
                    activated: true,
                },
            );
            ctx.constraint_type_map
                .insert(locked_id, ConstraintType::Lock);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(zwp_pointer_constraints::locked_pointer_locked_event(
                    locked_id,
                ));
                let _ = client.flush();
            }
        }
        zwp_pointer_constraints::constraints_request::CONFINE_POINTER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let confined_id = cursor_obj.new_id().unwrap_or(0);
            let surface_id = cursor_obj.object().unwrap_or(0);
            let pointer_id = cursor_obj.object().unwrap_or(0);
            let lifetime = cursor_obj.uint().unwrap_or(0);
            let _region = cursor_obj.object().ok();
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(
                    confined_id,
                    zwp_pointer_constraints::ZWP_CONFINED_POINTER_V1,
                    1,
                    ctx.client_id,
                );
            }
            ctx.pointer_constraints.insert(
                confined_id,
                PointerConstraint {
                    constraint_id: confined_id,
                    surface_id,
                    pointer_id,
                    lifetime,
                    region: None,
                    activated: true,
                },
            );
            ctx.constraint_type_map
                .insert(confined_id, ConstraintType::Confine);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(zwp_pointer_constraints::confined_pointer_confined_event(
                    confined_id,
                ));
                let _ = client.flush();
            }
        }
        zwp_pointer_constraints::constraints_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_locked_pointer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_pointer_constraints::locked_pointer_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.pointer_constraints.remove(&ctx.msg.sender_id);
            ctx.constraint_type_map.remove(&ctx.msg.sender_id);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(zwp_pointer_constraints::locked_pointer_unlocked_event(
                    ctx.msg.sender_id,
                ));
            }
        }
        zwp_pointer_constraints::locked_pointer_request::SET_CURSOR_POSITION_HINT => {
            let _ = ctx.msg;
        }
        zwp_pointer_constraints::locked_pointer_request::SET_REGION => {
            let _ = ctx.msg;
        }
        _ => {}
    }
}

pub fn handle_confined_pointer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_pointer_constraints::confined_pointer_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.pointer_constraints.remove(&ctx.msg.sender_id);
            ctx.constraint_type_map.remove(&ctx.msg.sender_id);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(zwp_pointer_constraints::confined_pointer_unconfined_event(
                    ctx.msg.sender_id,
                ));
            }
        }
        zwp_pointer_constraints::confined_pointer_request::SET_REGION => {
            let _ = ctx.msg;
        }
        _ => {}
    }
}

pub fn handle_relative_pointer_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_relative_pointer::relative_pointer_manager_request::GET_RELATIVE_POINTER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _pointer_id = cursor_obj.object().unwrap_or(0);
            let relative_pointer_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(
                    relative_pointer_id,
                    "zwp_relative_pointer_v1",
                    1,
                    ctx.client_id,
                );
            }
            ctx.client_relative_pointer_ids
                .insert(ctx.client_id, relative_pointer_id);
        }
        zwp_relative_pointer::relative_pointer_manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_relative_pointer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_relative_pointer::relative_pointer_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_pointer_gestures(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_pointer_gestures::pointer_gestures_request::GET_SWIPE_GESTURE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _pointer_id = cursor_obj.object().unwrap_or(0);
            let gesture_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(gesture_id, "zwp_gesture_swipe_v1", 1, ctx.client_id);
            }
        }
        zwp_pointer_gestures::pointer_gestures_request::GET_PINCH_GESTURE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _pointer_id = cursor_obj.object().unwrap_or(0);
            let gesture_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(gesture_id, "zwp_gesture_pinch_v1", 1, ctx.client_id);
            }
        }
        zwp_pointer_gestures::pointer_gestures_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_gesture(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_pointer_gestures::gesture_swipe_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_text_input_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_text_input::text_input_manager_request::CREATE_TEXT_INPUT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let text_input_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(text_input_id, "zwp_text_input_v1", 1, ctx.client_id);
            }
        }
        zwp_text_input::text_input_manager_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_text_input(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zwp_text_input::text_input_request::ACTIVATE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _seat_id = cursor_obj.object().unwrap_or(0);
            let surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(zwp_text_input::enter_event(ctx.msg.sender_id, surface_id));
            }
        }
        zwp_text_input::text_input_request::DEACTIVATE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(zwp_text_input::leave_event(ctx.msg.sender_id, surface_id));
            }
        }
        zwp_text_input::text_input_request::COMMIT_STATE => {}
        zwp_text_input::text_input_request::SHOW_INPUT_PANEL => {}
        zwp_text_input::text_input_request::HIDE_INPUT_PANEL => {}
        zwp_text_input::text_input_request::RESET => {}
        zwp_text_input::text_input_request::SET_SURROUNDING_TEXT => {}
        zwp_text_input::text_input_request::SET_CONTENT_TYPE => {}
        zwp_text_input::text_input_request::SET_CURSOR_RECTANGLE => {}
        zwp_text_input::text_input_request::SET_PREFERRED_LANGUAGE => {}
        zwp_text_input::text_input_request::INVOKE_ACTION => {}
        zwp_text_input::text_input_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_idle_inhibit_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zxdg_idle_inhibit::idle_inhibit_request::CREATE_INHIBITOR => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let surface_id = cursor_obj.object().unwrap_or(0);
            let inhibitor_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(inhibitor_id, "zwp_idle_inhibitor_v1", 1, ctx.client_id);
            }
            ctx.shell.add_idle_inhibitor(inhibitor_id, surface_id);
            eprintln!(
                "[edgerun-compositor] Idle inhibitor created for surface {}",
                surface_id
            );
        }
        zxdg_idle_inhibit::idle_inhibit_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_idle_inhibitor(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        zxdg_idle_inhibit::idle_inhibitor_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.shell.remove_idle_inhibitor(ctx.msg.sender_id);
        }
        _ => {}
    }
}
