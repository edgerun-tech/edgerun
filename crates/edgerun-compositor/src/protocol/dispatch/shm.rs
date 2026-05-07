//! wl_shm, wl_shm_pool, wl_buffer handlers.

use super::DispatchContext;
use crate::compositor::surface::ShmBufferInfo;
use crate::libc;
use crate::protocol::wl_shm;
use crate::wire::decode::ArgCursor;

pub fn handle_shm(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_shm::shm_request::CREATE_POOL => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let pool_id = cursor_obj.new_id().unwrap_or(0);
            let size = cursor_obj.int().unwrap_or(0);
            let fd = if !ctx.msg.fds.is_empty() {
                ctx.msg.fds[0]
            } else {
                -1
            };

            let client_pools = ctx
                .client_pool_map
                .entry(ctx.client_id)
                .or_insert_with(std::collections::HashMap::new);
            client_pools.insert(pool_id, 0);

            if let Ok(_internal_id) = ctx.shm.create_pool(pool_id, fd, size) {
                if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                    reg.register(pool_id, "wl_shm_pool", 1, ctx.client_id);
                }
            }
        }
        wl_shm::shm_request::RELEASE => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.client_shm_ids.remove(&ctx.client_id);
        }
        _ => {}
    }
}

pub fn handle_shm_pool(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_shm::shm_pool_request::CREATE_BUFFER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let buffer_id = cursor_obj.new_id().unwrap_or(0);
            let offset = cursor_obj.int().unwrap_or(0);
            let width = cursor_obj.int().unwrap_or(0);
            let height = cursor_obj.int().unwrap_or(0);
            let stride = cursor_obj.int().unwrap_or(0);
            let format = cursor_obj.uint().unwrap_or(0);

            let pool_fd = ctx
                .shm
                .pool_fd_by_client_id(ctx.msg.sender_id)
                .unwrap_or(-1);

            ctx.buffers.register(
                buffer_id,
                ShmBufferInfo {
                    pool_fd,
                    offset,
                    width,
                    height,
                    stride,
                    format,
                },
            );

            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(buffer_id, "wl_buffer", 1, ctx.client_id);
            }
        }
        wl_shm::shm_pool_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        wl_shm::shm_pool_request::RESIZE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let new_size = cursor_obj.int().unwrap_or(0);
            if let Some(pool) = ctx.shm.get_pool_by_client_id(ctx.msg.sender_id) {
                if new_size as usize > pool.size {
                    unsafe { libc::munmap(pool.mapping as *mut libc::c_void, pool.size) };
                    let new_ptr = unsafe {
                        libc::mmap(
                            std::ptr::null_mut(),
                            new_size as usize,
                            libc::PROT_READ,
                            libc::MAP_SHARED,
                            pool.fd,
                            0,
                        )
                    };
                    if new_ptr != libc::MAP_FAILED {
                        pool.mapping = new_ptr as *mut u8;
                        pool.size = new_size as usize;
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn handle_buffer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        wl_shm::buffer_request::DESTROY => {
            ctx.buffers.remove(ctx.msg.sender_id);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            super::send_delete_id(ctx.server, ctx.client_id, ctx.msg.sender_id);
        }
        _ => {}
    }
}
