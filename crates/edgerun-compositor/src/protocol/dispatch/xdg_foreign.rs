//! xdg foreign (export/import) handlers.

use super::DispatchContext;
use edgerun_protocols::wayland::decode::ArgCursor;
use edgerun_protocols::wayland::xdg_foreign;

pub fn handle_exporter(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_foreign::exporter_request::EXPORT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let exported_id = cursor_obj.new_id().unwrap_or(0);
            let _surface_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(exported_id, xdg_foreign::ZXDG_EXPORTED_V2, 1, ctx.client_id);
            }
            let handle = format!("{:x}-{:x}", ctx.client_id, exported_id);
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(xdg_foreign::exported_handle_event(exported_id, &handle));
                let _ = client.flush();
            }
        }
        xdg_foreign::exporter_request::DESTROY => {}
        _ => {}
    }
}

pub fn handle_exported(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_foreign::exported_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_importer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_foreign::importer_request::IMPORT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let imported_id = cursor_obj.new_id().unwrap_or(0);
            let _handle = cursor_obj.string().ok().flatten();
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(imported_id, xdg_foreign::ZXDG_IMPORTED_V2, 1, ctx.client_id);
            }
        }
        xdg_foreign::importer_request::DESTROY => {}
        _ => {}
    }
}

pub fn handle_imported(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        xdg_foreign::imported_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        xdg_foreign::imported_request::SET_PARENT_OF => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let child_surface_id = cursor_obj.object().unwrap_or(0);
            // Cross-client parent-child relationship: set the parent of child_surface_id
            // to the surface associated with this exported handle.
            // For now, log the relationship. Full validation requires tracking exported handles.
            eprintln!(
                "[edgerun-compositor] xdg_foreign SET_PARENT_OF: child_surface={}",
                child_surface_id
            );
        }
        _ => {}
    }
}
