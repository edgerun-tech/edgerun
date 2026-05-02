mod app_principal;
mod bootstrap;
mod wasm_host;

use anyhow::{Context, Result};
use clap::Parser;
use edgerun_proto::edgerun::v0::common::IdentityRef;
use edgerun_proto::edgerun::v0::stream::{CommandResultPayload, CommandType};
use prost::Message;
use std::sync::{Arc, Mutex};

#[cfg(feature = "gui")]
use eframe::egui;

#[cfg(feature = "gui")]
use edgerun_ui::NativeRenderer;

#[derive(Parser, Debug)]
#[command(name = "edgerun-app", about = "EdgeRun native UI runtime")]
struct Args {
    #[arg(default_value = "counter_app.wasm")]
    file: String,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long, default_value = "node-default")]
    node_id: String,

    #[arg(long)]
    settings_app: Option<String>,

    #[arg(long)]
    bootstrap_only: bool,
}

#[cfg(feature = "gui")]
struct App {
    renderer: NativeRenderer,
    wasm: Arc<Mutex<wasm_host::WasmRuntime>>,
    pending_ui: Arc<Mutex<Option<Vec<u8>>>>,
}

#[cfg(feature = "gui")]
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(bytes) = self.pending_ui.lock().unwrap().take() {
            self.renderer.set_ui(&bytes);
        }
        self.renderer.render(ctx);
    }
}

fn make_node_identity(node_id: &str) -> IdentityRef {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut h = DefaultHasher::new();
    node_id.hash(&mut h);
    let id_bytes = h.finish().to_le_bytes();

    IdentityRef {
        identity_id: id_bytes.to_vec(),
        identity_kind: Some(edgerun_proto::edgerun::v0::common::IdentityKind::Node as i32),
        key_hint: None,
    }
}

fn make_app_instance_id() -> [u8; 16] {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u128;
    now.to_le_bytes()
}

fn command_to_event_bytes(cmd: &edgerun_proto::edgerun::v0::stream::CommandEnvelope, accepted: bool) -> Vec<u8> {
    // Bootstrap simulation - creates event bytes without faking CommandResultPayload
    // Real nodes use record_and_respond_with_result_object which builds proper payloads
    let event_type = if accepted {
        edgerun_proto::edgerun::v0::stream::EventType::CommandCommitted
    } else {
        edgerun_proto::edgerun::v0::stream::EventType::CommandRejected
    };
    // Return minimal event bytes for bootstrap simulation only
    Vec::new()
}

fn run_bootstrap_loop(
    wasm_path: &str,
    verbose: bool,
    node_identity: IdentityRef,
) -> Result<bool> {
    let mut runtime = wasm_host::WasmRuntime::new(wasm_path, verbose)?;
    let app_key = Arc::new(app_principal::AppKeyPair::generate(b"settings-app".to_vec())?);
    let context = wasm_host::ExecutionContext {
        node_identity: node_identity.clone(),
        app_instance_id: make_app_instance_id(),
        delegation_chain: Vec::new(),
        app_key: app_key.clone(),
    };

    let mut bootstrap_state = bootstrap::BootstrapState::default();
    bootstrap_state.start();

    let mut event_results: Vec<Vec<u8>> = Vec::new();
    let mut iteration = 0;
    let max_iterations = 50;

    let action_sequence = [
        "action:start_setup",
        "action:generate_identity",
        "action:add_controller",
    ];

    for action in &action_sequence {
        if bootstrap_state.is_bootstrap_complete() {
            break;
        }

        iteration += 1;

        if verbose {
            eprintln!("Bootstrap iteration {}: {}", iteration, action);
        }

        let events = std::mem::take(&mut event_results);
        let result = runtime.run_with_events(action, context.clone(), events)?;

        if verbose && !result.ui_bytes.is_empty() {
            eprintln!("UI output: {} bytes", result.ui_bytes.len());
        }

        if result.pending_commands.is_empty() {
            if verbose {
                eprintln!("No commands emitted, continuing");
            }
            continue;
        }

        if verbose {
            eprintln!("Dispatching {} command(s)", result.pending_commands.len());
        }

        let dispatch_results =
            bootstrap::simulate_bootstrap_commands(result.pending_commands.clone(), &mut bootstrap_state);

            for (cmd, dispatch) in result.pending_commands.iter().zip(dispatch_results.iter()) {
                let cmd_type = CommandType::from_i32(cmd.command_type);
                let event_bytes = command_to_event_bytes(cmd, dispatch.accepted);
                event_results.push(event_bytes);

                if dispatch.accepted {
                    if verbose {
                        eprintln!("  command {:?} accepted", cmd_type);
                    }
                    match cmd_type {
                        Some(CommandType::CreateIdentity) | Some(CommandType::ImportIdentity) => {
                            bootstrap_state.mark_identity_created();
                        }
                        Some(CommandType::AddController) => {
                            bootstrap_state.mark_controller_added();
                        }
                        _ => {}
                    }
                } else {
                    if verbose {
                        eprintln!("  command {:?} rejected: {}", cmd_type, dispatch.reason);
                    }
                }
            }

        if bootstrap_state.is_bootstrap_complete() {
            bootstrap_state.complete();
            if verbose {
                eprintln!("Bootstrap complete after {} iteration(s)", iteration);
            }
            return Ok(true);
        }
    }

    if iteration >= max_iterations {
        eprintln!("Bootstrap exceeded max iterations ({}), aborting", max_iterations);
    }

    Ok(bootstrap_state.is_bootstrap_complete())
}

#[cfg(feature = "gui")]
fn run_gui_app(
    wasm_path: &str,
    verbose: bool,
    ctx: wasm_host::ExecutionContext,
) -> Result<()> {
    use eframe::egui;

    let wasm = wasm_host::WasmRuntime::new(wasm_path, verbose)?;
    let wasm = Arc::new(Mutex::new(wasm));

    if verbose {
        eprintln!(
            "ExecutionContext: node={:?}, instance={:?}",
            ctx.node_identity.identity_id, ctx.app_instance_id
        );
    }

    let initial_ui = wasm.lock().unwrap().run_with_context("", ctx.clone())?;

    if verbose {
        eprintln!("Initial UI bytes: {} bytes", initial_ui.len());
    }

    let pending_ui: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 500.0])
            .with_title("edgerun app"),
        ..Default::default()
    };

    let wasm_for_cb = wasm.clone();
    let pending_ui_for_cb = pending_ui.clone();
    let ctx_for_cb = Arc::new(ctx.clone());
    let verbose_for_cb = verbose;

    eframe::run_native(
        "edgerun app",
        options,
        Box::new(move |cc| {
            let egui_ctx = cc.egui_ctx.clone();
            let pending_ui_inner = pending_ui_for_cb.clone();
            let wasm_inner = wasm_for_cb.clone();
            let ctx_inner = ctx_for_cb.clone();

            let mut renderer = NativeRenderer::new();
            let ok = renderer.set_ui(&initial_ui);
            if verbose_for_cb {
                eprintln!("Initial set_ui: {} ({} bytes)", ok, initial_ui.len());
                if let Some(ui) = renderer.current_ui() {
                    eprintln!(
                        "Root node type: '{}', children: {}",
                        ui.node_type,
                        ui.children.len()
                    );
                    for (i, c) in ui.children.iter().enumerate() {
                        eprintln!(
                            "  child[{}]: type='{}', action={:?}",
                            i, c.node_type, c.action
                        );
                    }
                }
            }

            renderer.action_fn = Some(Box::new(move |action: String| {
                let payload = format!("action:{}", action);
                if verbose_for_cb {
                    eprintln!("Action: '{}', payload: '{}'", action, payload);
                }
                if let Ok(mut w) = wasm_inner.lock() {
                    if let Ok(ui_bytes) = w.run_with_context(&payload, (*ctx_inner).clone()) {
                        if verbose_for_cb {
                            eprintln!("UI response: {} bytes", ui_bytes.len());
                        }
                        *pending_ui_inner.lock().unwrap() = Some(ui_bytes);
                        egui_ctx.request_repaint();
                    }
                }
            }));

            Ok(Box::new(App {
                renderer,
                wasm,
                pending_ui,
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}

#[cfg(not(feature = "gui"))]
fn run_gui_app(
    wasm_path: &str,
    verbose: bool,
    ctx: wasm_host::ExecutionContext,
) -> Result<()> {
    let mut runtime = wasm_host::WasmRuntime::new(wasm_path, verbose)?;

    if verbose {
        eprintln!(
            "ExecutionContext: node={:?}, instance={:?}",
            ctx.node_identity.identity_id, ctx.app_instance_id
        );
    }

    let ui_bytes = runtime.run_with_context("", ctx.clone())?;

    if verbose {
        eprintln!("App UI output: {} bytes", ui_bytes.len());
    }

    eprintln!("GUI feature not enabled; app executed in headless mode");
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let node_identity = make_node_identity(&args.node_id);

    if args.settings_app.is_some() && !args.bootstrap_only {
        if args.verbose {
            eprintln!("Checking if bootstrap is needed...");
        }

        let bootstrap_ok = run_bootstrap_loop(
            args.settings_app.as_ref().unwrap(),
            args.verbose,
            node_identity.clone(),
        )?;

        if bootstrap_ok {
            if args.verbose {
                eprintln!("Bootstrap complete, proceeding to normal app execution");
            }
        } else {
            if args.verbose {
                eprintln!("Bootstrap did not complete, exiting");
            }
            return Ok(());
        }
    }

    if args.bootstrap_only {
        if args.verbose {
            eprintln!("Running in bootstrap-only mode");
        }
        let bootstrap_ok = run_bootstrap_loop(
            args.settings_app
                .as_ref()
                .context("--settings-app required with --bootstrap-only")?,
            args.verbose,
            node_identity.clone(),
        )?;

        if bootstrap_ok {
            eprintln!("Bootstrap complete");
        } else {
            eprintln!("Bootstrap did not complete");
        }
        return Ok(());
    }

    let app_key = Arc::new(app_principal::AppKeyPair::generate(b"default-app".to_vec())?);
    let ctx = wasm_host::ExecutionContext {
        node_identity,
        app_instance_id: make_app_instance_id(),
        delegation_chain: Vec::new(),
        app_key,
    };

    run_gui_app(&args.file, args.verbose, ctx)
}
