mod app;
mod event_loop;
mod wasmtime_mock;
mod wasmparser_mock {
    include!("wasmparser_mock.rs");
}

use anyhow::{Context, Result};
use edgerun_clap::Parser;
use edgerun_core::protocol::capability_check;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use wasmtime_mock::*;

#[derive(Parser, Debug)]
#[command(name = "edgerun-runtime", about = "Run EdgeRun WASM modules locally")]
struct Args {
    #[arg(default_value = "app.wasm")]
    file: String,

    #[arg(short, long, default_value = "")]
    input: String,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long, default_value = "")]
    events: String,

    #[arg(long)]
    listen: Option<u16>,

    #[arg(long)]
    timer: Option<u64>,
}

struct HostState {
    output: Arc<Mutex<Vec<u8>>>,
    messages: Arc<Mutex<Vec<(String, String)>>>,
    event_queue: Arc<Mutex<Vec<Vec<u8>>>>,
    exec_context: Option<Arc<Mutex<edgerun_core::protocol::ExecutionContext>>>,
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let wasm_path = Path::new(&args.file);
    if !wasm_path.exists() {
        anyhow::bail!("File not found: {}", args.file);
    }

    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm_path).context("Failed to load WASM module")?;

    if args.verbose {
        eprintln!("Loaded: {}", args.file);
    }

    let output = Arc::new(Mutex::new(Vec::new()));
    let messages = Arc::new(Mutex::new(Vec::new()));
    let event_queue = Arc::new(Mutex::new(parse_events(
        &args
            .events
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )));

    let host = HostState {
        output: output.clone(),
        messages: messages.clone(),
        event_queue: event_queue.clone(),
        exec_context: None,
        verbose: args.verbose,
    };

    let mut linker = Linker::new(&engine);

    let output_write = output.clone();
    let verbose = args.verbose;
    linker.func_wrap(
        "env",
        "write_output",
        move |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("No memory export");
            let data = mem.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            if end > data.len() {
                eprintln!("write_output: out of bounds ptr={} len={}", ptr, len);
                return;
            }
            let slice = &data[start..end];
            if verbose {
                eprintln!(
                    "write_output({}): {:?}",
                    slice.len(),
                    String::from_utf8_lossy(slice)
                );
            }
            let mut out = output_write.lock().unwrap();
            out.extend_from_slice(slice);
        },
    )?;

    let exec_ctx_for_blob = host.exec_context.clone();
    let verbose = args.verbose;
    linker.func_wrap(
        "env",
        "read_blob",
        move |mut _caller: Caller<'_, HostState>,
              _hash_ptr: i32,
              _hash_len: i32,
              _dst_ptr: i32|
              -> i32 {
            if let Some(ref ctx) = exec_ctx_for_blob {
                let ctx = ctx.lock().unwrap();
                let result =
                    app::check_capability(&ctx, capability_check::Operation::ReadBlob, None);
                if result.decision
                    != edgerun_core::protocol::capability_result::Decision::Granted as i32
                {
                    if verbose {
                        eprintln!("read_blob: capability denied");
                    }
                    return -3;
                }
            }
            -1
        },
    )?;

    let exec_ctx_for_wblob = host.exec_context.clone();
    let verbose = args.verbose;
    linker.func_wrap(
        "env",
        "write_blob",
        move |_caller: Caller<'_, HostState>, _ptr: i32, _len: i32, _hash_out_ptr: i32| -> i32 {
            if let Some(ref ctx) = exec_ctx_for_wblob {
                let ctx = ctx.lock().unwrap();
                let result =
                    app::check_capability(&ctx, capability_check::Operation::WriteBlob, None);
                if result.decision
                    != edgerun_core::protocol::capability_result::Decision::Granted as i32
                {
                    if verbose {
                        eprintln!("write_blob: capability denied");
                    }
                    return -3;
                }
            }
            -1
        },
    )?;

    let exec_ctx_for_msg = host.exec_context.clone();
    let verbose = args.verbose;
    let msg_write = messages.clone();
    linker.func_wrap(
        "env",
        "send_message",
        move |mut caller: Caller<'_, HostState>,
              target_ptr: i32,
              target_len: i32,
              payload_ptr: i32,
              payload_len: i32|
              -> i32 {
            if let Some(ref ctx) = exec_ctx_for_msg {
                let ctx = ctx.lock().unwrap();
                let result =
                    app::check_capability(&ctx, capability_check::Operation::SendMessage, None);
                if result.decision
                    != edgerun_core::protocol::capability_result::Decision::Granted as i32
                {
                    if verbose {
                        eprintln!("send_message: capability denied");
                    }
                    return -3;
                }
            }

            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("No memory export");
            let data = mem.data(&caller);
            let safe_read = |p: i32, l: i32| -> String {
                let start = p as usize;
                let end = start + l as usize;
                if end > data.len() {
                    String::from("<out of bounds>")
                } else {
                    String::from_utf8_lossy(&data[start..end]).to_string()
                }
            };
            let target = safe_read(target_ptr, target_len);
            let payload = safe_read(payload_ptr, payload_len);
            if verbose {
                eprintln!(
                    "send_message(target={}, payload_size={})",
                    target,
                    payload.len()
                );
            }
            msg_write.lock().unwrap().push((target, payload));
            0
        },
    )?;

    let evt_queue = event_queue.clone();
    let verbose = args.verbose;
    linker.func_wrap(
        "env",
        "poll_event",
        move |mut caller: Caller<'_, HostState>, buf_ptr: i32, buf_len: i32| -> i32 {
            let mut queue = evt_queue.lock().unwrap();
            if queue.is_empty() {
                return -1;
            }
            let event_data = queue.remove(0);
            let event_len = event_data.len() as i32;
            if event_len > buf_len {
                if verbose {
                    eprintln!("poll_event: buffer too small ({} > {})", event_len, buf_len);
                }
                queue.insert(0, event_data);
                return -2;
            }
            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("No memory export");
            let mem_data = mem.data_mut(&mut caller);
            let start = buf_ptr as usize;
            let end = start + event_len as usize;
            if end > mem_data.len() {
                if verbose {
                    eprintln!(
                        "poll_event: out of bounds ptr={} len={}",
                        buf_ptr, event_len
                    );
                }
                queue.insert(0, event_data);
                return -1;
            }
            mem_data[start..end].copy_from_slice(&event_data);
            if verbose {
                eprintln!(
                    "poll_event: type={}, subtype={}",
                    event_data[0],
                    if event_data.len() > 1 {
                        event_data[1]
                    } else {
                        0
                    }
                );
            }
            event_len
        },
    )?;

    let mut store = Store::new(&engine, host);
    let instance = linker.instantiate(&mut store, &module)?;

    let run_func = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "run")
        .context("Failed to find 'run' function")?;

    if args.verbose {
        eprintln!("Found entry point: run(ptr, len)");
    }

    if args.listen.is_some() || args.timer.is_some() {
        run_event_driven(
            &args,
            &mut store,
            run_func,
            event_queue.clone(),
            output.clone(),
            messages.clone(),
        )?;
    } else {
        run_batch(
            &args,
            &mut store,
            instance,
            run_func,
            output.clone(),
            messages.clone(),
        )?;
    }

    Ok(())
}

fn run_batch(
    args: &Args,
    store: &mut Store<HostState>,
    instance: Instance,
    mut run_func: TypedFunc<(i32, i32), i32>,
    output: Arc<Mutex<Vec<u8>>>,
    messages: Arc<Mutex<Vec<(String, String)>>>,
) -> Result<()> {
    let input_bytes = args.input.as_bytes();
    let input_len = input_bytes.len() as i32;

    if input_len > 0 {
        let memory = instance
            .get_memory(&mut *store, "memory")
            .context("No memory export")?;

        let offset = memory.data_size(&mut *store) as i32;
        memory.grow(&mut *store, 1)?;

        let mem_data = memory.data_mut(&mut *store);
        let start = offset as usize;
        let end = start + input_bytes.len();
        if end > mem_data.len() {
            anyhow::bail!("Input too large for WASM memory");
        }
        mem_data[start..end].copy_from_slice(input_bytes);

        if args.verbose {
            eprintln!("Input: {} bytes at ptr={}", input_len, offset);
        }

        let result = run_func.call(&mut *store, (offset, input_len))?;
        if args.verbose {
            eprintln!("run() returned: {}", result);
        }
    } else {
        if args.verbose {
            eprintln!("Input: empty");
        }
        let result = run_func.call(&mut *store, (0, 0))?;
        if args.verbose {
            eprintln!("run() returned: {}", result);
        }
    }

    drop(run_func);
    let _ = store;

    print_output(&output, &messages, args.verbose);
    Ok(())
}

fn run_event_driven(
    args: &Args,
    store: &mut Store<HostState>,
    mut run_func: TypedFunc<(i32, i32), i32>,
    event_queue: Arc<Mutex<Vec<Vec<u8>>>>,
    output: Arc<Mutex<Vec<u8>>>,
    _messages: Arc<Mutex<Vec<(String, String)>>>,
) -> Result<()> {
    let mut event_loop = event_loop::EventLoop::new(event_queue.clone(), args.verbose)
        .context("Failed to create event loop")?;

    if let Some(port) = args.listen {
        event_loop
            .add_tcp_listener(port)
            .context("Failed to add TCP listener")?;
    }

    if let Some(interval_ms) = args.timer {
        event_loop
            .add_timer(interval_ms)
            .context("Failed to add timer")?;
    }

    if args.verbose {
        eprintln!("Event-driven mode: epoll loop started");
    }

    let mut run_count = 0u64;
    loop {
        let events_fired = event_loop.run_once(100).context("epoll_wait failed")?;

        if events_fired > 0 {
            output.lock().unwrap().clear();
            let result = run_func.call(&mut *store, (0, 0))?;
            run_count += 1;

            if args.verbose {
                eprintln!("WASM run #{} returned: {}", run_count, result);
            }

            let out = output.lock().unwrap();
            if !out.is_empty() {
                if out.len() >= 10 {
                    let status = u16::from_le_bytes([out[0], out[1]]);
                    let ct_len = u32::from_le_bytes([out[2], out[3], out[4], out[5]]) as usize;
                    let content_type = if ct_len > 0 && 6 + ct_len <= out.len() {
                        String::from_utf8_lossy(&out[6..6 + ct_len]).to_string()
                    } else {
                        String::new()
                    };
                    let body_offset = 6 + ct_len;
                    if body_offset + 4 <= out.len() {
                        let body_len = u32::from_le_bytes([
                            out[body_offset],
                            out[body_offset + 1],
                            out[body_offset + 2],
                            out[body_offset + 3],
                        ]) as usize;
                        let body_start = body_offset + 4;
                        let body_end = body_start.saturating_add(body_len).min(out.len());
                        let body = &out[body_start..body_end];
                        if !args.verbose {
                            println!("{}", String::from_utf8_lossy(body));
                        }
                    }
                } else if !args.verbose {
                    println!("{}", String::from_utf8_lossy(&out));
                }
            }
        }
    }
}

fn print_output(
    output: &Arc<Mutex<Vec<u8>>>,
    messages: &Arc<Mutex<Vec<(String, String)>>>,
    verbose: bool,
) {
    let out = output.lock().unwrap();
    if !out.is_empty() {
        if out.len() >= 10 {
            let status = u16::from_le_bytes([out[0], out[1]]);
            let ct_len = u32::from_le_bytes([out[2], out[3], out[4], out[5]]) as usize;
            let content_type = if ct_len > 0 && 6 + ct_len <= out.len() {
                String::from_utf8_lossy(&out[6..6 + ct_len]).to_string()
            } else {
                String::new()
            };
            let body_offset = 6 + ct_len;
            if body_offset + 4 <= out.len() {
                let body_len = u32::from_le_bytes([
                    out[body_offset],
                    out[body_offset + 1],
                    out[body_offset + 2],
                    out[body_offset + 3],
                ]) as usize;
                let body_start = body_offset + 4;
                let body_end = body_start.saturating_add(body_len).min(out.len());
                let body = &out[body_start..body_end];
                if verbose {
                    eprintln!("\n--- Response ---");
                    eprintln!("Status: {}", status);
                    if !content_type.is_empty() {
                        eprintln!("Content-Type: {}", content_type);
                    }
                    eprintln!("Body ({} bytes):", body.len());
                    eprintln!("{}", String::from_utf8_lossy(body));
                    eprintln!("--- End Response ---");
                } else {
                    print!("{}", String::from_utf8_lossy(body));
                }
            } else {
                println!("{}", String::from_utf8_lossy(&out));
            }
        } else {
            println!("{}", String::from_utf8_lossy(&out));
        }
    }

    let msgs = messages.lock().unwrap();
    if verbose && !msgs.is_empty() {
        for (target, payload) in msgs.iter() {
            eprintln!("  -> {} : {}", target, payload);
        }
    }
}

fn parse_events(args: &[String]) -> Vec<Vec<u8>> {
    let mut events = Vec::new();
    for arg in args {
        if let Some(event) = parse_single_event(arg) {
            events.push(event);
        }
    }
    events
}

fn parse_single_event(s: &str) -> Option<Vec<u8>> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.is_empty() {
        return None;
    }
    match parts[0] {
        "net" => parse_net_event(&parts[1..]),
        "disk" => parse_disk_event(&parts[1..]),
        "timer" => parse_timer_event(&parts[1..]),
        _ => None,
    }
}

fn parse_net_event(parts: &[&str]) -> Option<Vec<u8>> {
    if parts.is_empty() {
        return None;
    }
    let subtype = match parts[0] {
        "connect" => 1u8,
        "disconnect" => 2u8,
        "recv" => 3u8,
        "error" => 4u8,
        _ => return None,
    };
    let sock_id: u32 = if parts.len() > 1 {
        parts[1].parse().ok()?
    } else {
        0
    };
    let mut data = Vec::new();
    data.extend_from_slice(&sock_id.to_le_bytes());
    data.push(subtype);
    if subtype == 3 && parts.len() > 2 {
        let payload = parts[2].as_bytes();
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
    }
    let mut event = vec![1u8];
    event.extend_from_slice(&data);
    Some(event)
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).ok()?;
        bytes.push(byte);
    }
    Some(bytes)
}

fn parse_disk_event(parts: &[&str]) -> Option<Vec<u8>> {
    if parts.is_empty() {
        return None;
    }
    let subtype = match parts[0] {
        "read" => 1u8,
        "write" => 2u8,
        "error" => 3u8,
        _ => return None,
    };
    let op_id: u32 = if parts.len() > 1 {
        parts[1].parse().ok()?
    } else {
        0
    };
    let mut data = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.push(subtype);
    if (subtype == 1 || subtype == 2) && parts.len() > 2 {
        let hash = decode_hex(parts[2])?;
        if hash.len() != 32 {
            return None;
        }
        data.extend_from_slice(&hash);
        if subtype == 1 && parts.len() > 3 {
            data.extend_from_slice(parts[3].as_bytes());
        }
    }
    let mut event = vec![2u8];
    event.extend_from_slice(&data);
    Some(event)
}

fn parse_timer_event(parts: &[&str]) -> Option<Vec<u8>> {
    let timer_id: u64 = if !parts.is_empty() {
        parts[0].parse().ok()?
    } else {
        1
    };
    let mut data = vec![3u8, 1u8];
    data.extend_from_slice(&timer_id.to_le_bytes());
    Some(data)
}
