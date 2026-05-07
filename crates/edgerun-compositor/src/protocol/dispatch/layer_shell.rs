//! zwlr_layer_shell_v1 dispatch handlers.
//!
//! Handles get_layer_surface, surface configuration, anchoring,
//! exclusive zones, margins, layer changes, and popup attachment.

use super::DispatchContext;
use edgerun_protocols::wayland::decode::ArgCursor;
use edgerun_protocols::wayland::layer_shell;

/// Layer surface state for anchor/size/keyboard interactivity.
/// Stored in the Shell alongside toplevels and popups.
pub fn handle_layer_shell(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        layer_shell::layer_shell_request::GET_LAYER_SURFACE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let layer_surface_id = cursor_obj.new_id().unwrap_or(0);
            let _surface_id = cursor_obj.object().unwrap_or(0);
            let _output_id = cursor_obj.object(); // nullable wl_output
            let layer = cursor_obj.uint().unwrap_or(0);
            let _namespace = cursor_obj.string().unwrap_or_default();

            // Validate layer
            if layer > 3 {
                // Send protocol error: invalid_layer
                return;
            }

            // Register the layer surface object
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(layer_surface_id, "zwlr_layer_surface_v1", 4, ctx.client_id);
            }

            // Create layer surface in shell
            let anchor = 0u32;
            let exclusive_zone = 0i32;
            let margin_top = 0i32;
            let margin_right = 0i32;
            let margin_bottom = 0i32;
            let margin_left = 0i32;
            let keyboard_interactivity = 0u32;
            let desired_width = 0u32;
            let desired_height = 0u32;
            ctx.shell.create_layer_surface(
                layer_surface_id,
                _surface_id,
                layer,
                anchor,
                exclusive_zone,
                margin_top,
                margin_right,
                margin_bottom,
                margin_left,
                keyboard_interactivity,
                desired_width,
                desired_height,
            );
            // Track client_id
            if let Some(ls) = ctx.shell.layer_surfaces.get_mut(&layer_surface_id) {
                ls.client_id = ctx.client_id;
            }

            // Send initial configure
            let output_w = ctx.shell.output_width as u32;
            let output_h = ctx.shell.output_height as u32;
            let serial = ctx
                .shell
                .configure_layer_surface(layer_surface_id, output_w, output_h);
            ctx.send_flush(layer_shell::layer_surface_configure_event(
                layer_surface_id,
                serial,
                output_w,
                output_h,
            ));
        }
        layer_shell::layer_shell_request::DESTROY => {
            // Destroy all layer surfaces owned by this client
            let ids: Vec<u32> = ctx.shell.layer_surfaces_for_client(ctx.client_id);
            for id in ids {
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(layer_shell::layer_surface_closed_event(id));
                }
                ctx.shell.destroy_layer_surface(id);
                super::send_delete_id(ctx.server, ctx.client_id, id);
            }
        }
        _ => {}
    }
}

pub fn handle_layer_surface(ctx: &mut DispatchContext) {
    let surface_id = ctx.msg.sender_id;
    match ctx.msg.opcode {
        layer_shell::layer_surface_request::SET_SIZE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let width = cursor_obj.uint().unwrap_or(0);
            let height = cursor_obj.uint().unwrap_or(0);
            ctx.shell.set_layer_surface_size(surface_id, width, height);
        }
        layer_shell::layer_surface_request::SET_ANCHOR => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let anchor = cursor_obj.uint().unwrap_or(0);
            ctx.shell.set_layer_surface_anchor(surface_id, anchor);
        }
        layer_shell::layer_surface_request::SET_EXCLUSIVE_ZONE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let zone = cursor_obj.int().unwrap_or(0);
            ctx.shell.set_layer_surface_exclusive_zone(surface_id, zone);
        }
        layer_shell::layer_surface_request::SET_MARGIN => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let top = cursor_obj.int().unwrap_or(0);
            let right = cursor_obj.int().unwrap_or(0);
            let bottom = cursor_obj.int().unwrap_or(0);
            let left = cursor_obj.int().unwrap_or(0);
            ctx.shell
                .set_layer_surface_margin(surface_id, top, right, bottom, left);
        }
        layer_shell::layer_surface_request::SET_KEYBOARD_INTERACTIVITY => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let value = cursor_obj.uint().unwrap_or(0);
            ctx.shell
                .set_layer_surface_keyboard_interactivity(surface_id, value);
        }
        layer_shell::layer_surface_request::GET_POPUP => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let _popup_id = cursor_obj.object().unwrap_or(0);
            // Attach xdg_popup to this layer surface — handled by xdg_popup handler
            // via xdg_surface_to_wl_surface mapping
            ctx.shell.set_layer_popup_parent(surface_id, _popup_id);
        }
        layer_shell::layer_surface_request::ACK_CONFIGURE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let serial = cursor_obj.uint().unwrap_or(0);
            ctx.shell.ack_layer_configure(surface_id, serial);
        }
        layer_shell::layer_surface_request::SET_LAYER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let layer = cursor_obj.uint().unwrap_or(0);
            if layer > 3 {
                // Protocol error: invalid_layer
                return;
            }
            ctx.shell.set_layer_surface_layer(surface_id, layer);
        }
        layer_shell::layer_surface_request::DESTROY => {
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(layer_shell::layer_surface_closed_event(surface_id));
            }
            ctx.shell.destroy_layer_surface(surface_id);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(surface_id);
            }
            super::send_delete_id(ctx.server, ctx.client_id, surface_id);
        }
        _ => {}
    }
}
