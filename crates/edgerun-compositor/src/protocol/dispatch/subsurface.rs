//! wl_subcompositor, wl_subsurface handlers.

use super::DispatchContext;
use edgerun_protocols::wayland::decode::ArgCursor;
use edgerun_protocols::wayland::wl_subcompositor;

pub fn handle_subcompositor(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_subcompositor::subcompositor_request::GET_SUBSURFACE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let subsurface_id = cursor_obj.new_id().unwrap_or(0);
            let surface_id = cursor_obj.object().unwrap_or(0);
            let parent_id = cursor_obj.object().unwrap_or(0);

            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(subsurface_id, "wl_subsurface", 1, ctx.client_id);
            }

            ctx.shell.subsurfaces.create(surface_id, parent_id);
        }
        wl_subcompositor::subcompositor_request::DESTROY => {
            // Destroying wl_subcompositor is fine — just clean up the registry
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_subcompositor_ids.remove(&ctx.client_id);
        }
        _ => {}
    }
}

pub fn handle_subsurface(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_subcompositor::subsurface_request::SET_POSITION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);

            ctx.shell.subsurfaces.set_position(ctx.msg.sender_id, x, y);
        }
        wl_subcompositor::subsurface_request::SET_SYNC => {
            if let Some(sub) = ctx.shell.subsurfaces.get_mut(ctx.msg.sender_id) {
                sub.sync = true;
            }
        }
        wl_subcompositor::subsurface_request::SET_DESYNC => {
            if let Some(sub) = ctx.shell.subsurfaces.get_mut(ctx.msg.sender_id) {
                sub.sync = false;
            }
        }
        wl_subcompositor::subsurface_request::PLACE_ABOVE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let sibling_id = cursor_obj.object().unwrap_or(0);
            ctx.shell
                .subsurfaces
                .place_above(ctx.msg.sender_id, sibling_id);
        }
        wl_subcompositor::subsurface_request::PLACE_BELOW => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let sibling_id = cursor_obj.object().unwrap_or(0);
            ctx.shell
                .subsurfaces
                .place_below(ctx.msg.sender_id, sibling_id);
        }
        wl_subcompositor::subsurface_request::DESTROY => {
            ctx.shell.subsurfaces.destroy(ctx.msg.sender_id);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}
