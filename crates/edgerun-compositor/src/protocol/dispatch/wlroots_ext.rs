//! wlroots extension handlers: screencopy, primary_selection, data_control.

use super::DispatchContext;
use crate::protocol::screencopy;
use crate::protocol::screencopy::ScreencopyFrame;
use crate::protocol::primary_selection;
use crate::protocol::data_control;
use crate::protocol::wl_data_device;
use crate::wire::decode::ArgCursor;

pub fn handle_screencopy_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        screencopy::manager_request::CAPTURE_OUTPUT => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let frame_id = cursor_obj.new_id().unwrap_or(0);
            let _overlay_cursor = cursor_obj.uint().unwrap_or(0);
            let _capture_type = cursor_obj.uint().unwrap_or(0);
            let _output_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(frame_id, screencopy::ZWLR_SCREENCOPY_FRAME_V1, 3, ctx.client_id);
            }
            ctx.screencopy_state.pending_frames.insert(frame_id, ScreencopyFrame {
                frame_id, client_id: ctx.client_id,
                width: ctx.screencopy_state.width, height: ctx.screencopy_state.height,
                stride: ctx.screencopy_state.stride, format: 0x34325258,
                region: None, copy_requested: false,
                target_pool_fd: None, target_offset: 0, target_size: 0,
            });
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(screencopy::frame_buffer_event(
                    frame_id, 0x34325258,
                    ctx.screencopy_state.width, ctx.screencopy_state.height, ctx.screencopy_state.stride));
                let _ = client.flush();
            }
        }
        screencopy::manager_request::CAPTURE_OUTPUT_REGION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let frame_id = cursor_obj.new_id().unwrap_or(0);
            let _overlay_cursor = cursor_obj.uint().unwrap_or(0);
            let _capture_type = cursor_obj.uint().unwrap_or(0);
            let _output_id = cursor_obj.object().unwrap_or(0);
            let x = cursor_obj.int().unwrap_or(0);
            let y = cursor_obj.int().unwrap_or(0);
            let w = cursor_obj.int().unwrap_or(0);
            let h = cursor_obj.int().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(frame_id, screencopy::ZWLR_SCREENCOPY_FRAME_V1, 3, ctx.client_id);
            }
            let capture_w = if w > 0 { w as u32 } else { ctx.screencopy_state.width };
            let capture_h = if h > 0 { h as u32 } else { ctx.screencopy_state.height };
            let capture_stride = capture_w * 4;
            ctx.screencopy_state.pending_frames.insert(frame_id, ScreencopyFrame {
                frame_id, client_id: ctx.client_id,
                width: capture_w, height: capture_h, stride: capture_stride, format: 0x34325258,
                region: Some((x, y, w, h)), copy_requested: false,
                target_pool_fd: None, target_offset: 0, target_size: 0,
            });
            if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                client.send_message(screencopy::frame_buffer_event(
                    frame_id, 0x34325258, capture_w, capture_h, capture_stride));
                let _ = client.flush();
            }
        }
        screencopy::manager_request::DESTROY => {}
        _ => {}
    }
}

pub fn handle_screencopy_frame(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        screencopy::frame_request::COPY | screencopy::frame_request::COPY_WITH_DAMAGE => {
            let buffer_id = ArgCursor::from_message(&ctx.msg).object().unwrap_or(0);
            if buffer_id == 0 {
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(screencopy::frame_failed_event(ctx.msg.sender_id));
                }
                return;
            }
            if let Some(buffer_info) = ctx.buffers.get(buffer_id) {
                if let Some(frame) = ctx.screencopy_state.pending_frames.get(&ctx.msg.sender_id) {
                    if frame.copy_requested {
                        if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                            client.send_message(screencopy::frame_failed_event(ctx.msg.sender_id));
                        }
                        return;
                    }
                    let src_pixels = &ctx.screencopy_state.pixels;
                    let fb_width = ctx.screencopy_state.width as usize;
                    let fb_height = ctx.screencopy_state.height as usize;
                    let fb_stride = ctx.screencopy_state.stride as usize;
                    let buf_width = buffer_info.width as usize;
                    let buf_height = buffer_info.height as usize;
                    let buf_stride = buffer_info.stride as usize;
                    let pool_fd = buffer_info.pool_fd;
                    let offset = buffer_info.offset as usize;
                    if !src_pixels.is_empty() && pool_fd >= 0 {
                        let map_size = buf_stride * buf_height;
                        let buf_ptr = unsafe {
                            libc::mmap(std::ptr::null_mut(), map_size,
                                libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED,
                                pool_fd, offset as libc::off_t)
                        };
                        if buf_ptr != libc::MAP_FAILED {
                            let dst = unsafe { std::slice::from_raw_parts_mut(buf_ptr as *mut u8, map_size) };
                            let copy_w = fb_width.min(buf_width);
                            let copy_h = fb_height.min(buf_height);
                            for row in 0..copy_h {
                                let src_row = row * fb_stride;
                                let dst_row = row * buf_stride;
                                let copy_bytes = copy_w * 4;
                                if src_row + copy_bytes <= src_pixels.len() && dst_row + copy_bytes <= dst.len() {
                                    dst[dst_row..dst_row + copy_bytes]
                                        .copy_from_slice(&src_pixels[src_row..src_row + copy_bytes]);
                                }
                            }
                            unsafe { libc::munmap(buf_ptr, map_size) };
                        }
                    }
                    if let Some(frame_mut) = ctx.screencopy_state.pending_frames.get_mut(&ctx.msg.sender_id) {
                        frame_mut.copy_requested = true;
                    }
                    if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                        client.send_message(screencopy::frame_flags_event(
                            ctx.msg.sender_id, screencopy::frame_flags::Y_INVERT));
                        client.send_message(screencopy::frame_ready_event(ctx.msg.sender_id));
                        let _ = client.flush();
                    }
                }
            } else {
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(screencopy::frame_failed_event(ctx.msg.sender_id));
                }
            }
        }
        screencopy::frame_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            ctx.screencopy_state.pending_frames.remove(&ctx.msg.sender_id);
        }
        _ => {}
    }
}

pub fn handle_primary_selection_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        primary_selection::manager_request::CREATE_DATA_SOURCE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let source_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(source_id, primary_selection::ZWLR_PRIMARY_SELECTION_SOURCE_V1, 1, ctx.client_id);
            }
        }
        primary_selection::manager_request::GET_PRIMARY_SELECTION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let device_id = cursor_obj.new_id().unwrap_or(0);
            let _seat_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(device_id, primary_selection::ZWLR_PRIMARY_SELECTION_DEVICE_V1, 1, ctx.client_id);
            }
            if let Some(ref source) = *ctx.current_primary_selection {
                *ctx.primary_selection_offer_counter += 1;
                let offer_id = *ctx.primary_selection_offer_counter;
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(primary_selection::device_selection_event(device_id, offer_id));
                    for mime in &source.mime_types {
                        client.send_message(primary_selection::offer_offer_event(offer_id, mime));
                    }
                }
            } else {
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(primary_selection::device_selection_event(device_id, 0));
                }
            }
        }
        primary_selection::manager_request::DESTROY => {}
        _ => {}
    }
}

pub fn handle_primary_selection_device(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        primary_selection::device_request::SET_SELECTION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let source_id = cursor_obj.object().unwrap_or(0);
            if source_id != 0 {
                *ctx.primary_selection_offer_counter += 1;
                *ctx.current_primary_selection = Some(crate::protocol::dispatch::PrimarySelectionSource {
                    id: source_id, owner_client_id: ctx.client_id, mime_types: Vec::new(),
                });
            } else {
                *ctx.current_primary_selection = None;
            }
        }
        primary_selection::device_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_primary_selection_offer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        primary_selection::offer_request::RECEIVE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(mime_type)) = cursor_obj.string() {
                let fd = if !ctx.msg.fds.is_empty() { ctx.msg.fds[0] } else { -1 };
                if fd >= 0 {
                    if let Some(ref source) = *ctx.current_primary_selection {
                        if let Some(source_client) = ctx.server.client_mut(source.owner_client_id) {
                            source_client.send_message(primary_selection::source_send_event(
                                source.id, &mime_type.0, fd));
                        }
                    }
                }
            }
        }
        primary_selection::offer_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_primary_selection_source(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        primary_selection::source_request::OFFER => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(mime_type)) = cursor_obj.string() {
                if let Some(source) = ctx.current_primary_selection.as_mut() {
                    source.mime_types.push(mime_type.0);
                }
            }
        }
        primary_selection::source_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
            if let Some(ref source) = *ctx.current_primary_selection {
                if source.id == ctx.msg.sender_id {
                    *ctx.current_primary_selection = None;
                }
            }
        }
        _ => {}
    }
}

pub fn handle_data_control_manager(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        data_control::manager_request::CREATE_DATA_SOURCE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let source_id = cursor_obj.new_id().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(source_id, data_control::ZWLR_DATA_CONTROL_SOURCE_V1, 1, ctx.client_id);
            }
        }
        data_control::manager_request::GET_DATA_DEVICE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let device_id = cursor_obj.new_id().unwrap_or(0);
            let _seat_id = cursor_obj.object().unwrap_or(0);
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.register(device_id, data_control::ZWLR_DATA_CONTROL_DEVICE_V1, 2, ctx.client_id);
            }
            if let Some(ref _source) = *ctx.current_data_source {
                let offer_id = *ctx.selection_offer_counter + 1;
                if let Some(client) = ctx.server.client_mut(ctx.client_id) {
                    client.send_message(data_control::data_control_device_data_offer_event(device_id, offer_id));
                    client.send_message(data_control::data_control_device_selection_event(device_id, offer_id));
                    if let Some(ref source) = *ctx.current_data_source {
                        for mime in &source.mime_types {
                            client.send_message(data_control::data_control_offer_event(offer_id, mime));
                        }
                    }
                }
            }
        }
        data_control::manager_request::DESTROY => {}
        _ => {}
    }
}

pub fn handle_data_control_device(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        data_control::data_control_device_request::SET_SELECTION => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            let source_id = cursor_obj.object().unwrap_or(0);
            if source_id != 0 {
                *ctx.selection_offer_counter += 1;
                let offer_id = *ctx.selection_offer_counter;
                *ctx.current_data_source = Some(crate::protocol::dispatch::DataSource {
                    id: source_id, owner_client_id: ctx.client_id, mime_types: Vec::new(),
                });
                for (&cid, &device_id) in ctx.client_data_device_ids.iter() {
                    if let Some(client) = ctx.server.client_mut(cid) {
                        client.send_message(data_control::data_control_device_data_offer_event(device_id, offer_id));
                        client.send_message(data_control::data_control_device_selection_event(device_id, offer_id));
                    }
                }
            }
        }
        data_control::data_control_device_request::RELEASE => {}
        _ => {}
    }
}

pub fn handle_data_control_offer(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        data_control::data_control_offer_request::RECEIVE => {
            let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
            if let Ok(Some(mime_type)) = cursor_obj.string() {
                let fd = if !ctx.msg.fds.is_empty() { ctx.msg.fds[0] } else { -1 };
                if fd >= 0 {
                    if let Some(ref source) = *ctx.current_data_source {
                        if let Some(source_client) = ctx.server.client_mut(source.owner_client_id) {
                            source_client.send_message(wl_data_device::data_source_send_event(
                                source.id, &mime_type.0, fd));
                        }
                    }
                }
            }
        }
        data_control::data_control_offer_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}

pub fn handle_data_control_source(ctx: &mut DispatchContext) {
    match ctx.msg.opcode {
        data_control::data_control_source_request::OFFER => {
            if let Some(source) = ctx.current_data_source {
                let mut cursor_obj = ArgCursor::from_message(&ctx.msg);
                if let Ok(Some(mime_type)) = cursor_obj.string() {
                    source.mime_types.push(mime_type.0);
                }
            }
        }
        data_control::data_control_source_request::DESTROY => {
            if let Some(reg) = ctx.client_registries.get_mut(&ctx.client_id) {
                reg.destroy(ctx.msg.sender_id);
            }
        }
        _ => {}
    }
}
