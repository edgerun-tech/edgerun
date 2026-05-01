mod wasm_host;

use anyhow::{Context, Result};
use clap::Parser;
use eframe::egui;
use edgerun_proto::edgerun::v0::common::IdentityRef;
use edgerun_ui::NativeRenderer;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(name = "edgerun-app", about = "EdgeRun native UI runtime")]
struct Args {
    #[arg(default_value = "counter_app.wasm")]
    file: String,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long, default_value = "node-default")]
    node_id: String,
}

struct App {
    renderer: NativeRenderer,
    wasm: Arc<Mutex<wasm_host::WasmRuntime>>,
    pending_ui: Arc<Mutex<Option<Vec<u8>>>>,
}

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

fn main() -> Result<()> {
    let args = Args::parse();

    let wasm = wasm_host::WasmRuntime::new(&args.file, args.verbose)?;
    let wasm = Arc::new(Mutex::new(wasm));

    let ctx = wasm_host::ExecutionContext {
        node_identity: make_node_identity(&args.node_id),
        app_instance_id: make_app_instance_id(),
        delegation_chain: Vec::new(),
    };

    if args.verbose {
        eprintln!(
            "ExecutionContext: node={:?}, instance={:?}",
            ctx.node_identity.identity_id, ctx.app_instance_id
        );
    }

    let initial_ui = wasm.lock().unwrap().run_with_context("", ctx.clone())?;

    if args.verbose {
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
    let verbose_for_cb = args.verbose;

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
