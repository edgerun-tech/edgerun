use std::env;

use edgerun_ui_core::gpu::sdl::{SdlGlWindowOptions, instantiate_sdl_app};
use edgerun_ui_core::gpu::{
    Color4, FontAtlas, GpuScene, UiAction, UiAppControl, UiColorScheme, UiNode, UiPainter, UiRect,
    UiShellState, UiSurfaceApp, UiWorkspace, UiWorkspaceSurface, column, palette, text,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("edgerun-frontend: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    if args.dump_scene {
        let mut workspace = UiWorkspace::single(UiWorkspaceSurface::new(1, "EdgeRun"));
        let mut scene = GpuScene::new(palette::BG);
        {
            let mut ui = UiPainter::new(&mut scene);
            workspace.render(
                &mut ui,
                UiRect::new(0.0, 0.0, 1120.0, 720.0),
                |ui, bounds, _surface| {
                    ui.fill_rect(bounds, 8.0, palette::PANEL);
                    ui.bounded_label(bounds.x + 12.0, bounds.y + 14.0, bounds.w - 24.0, "EdgeRun", 2.0, palette::TEXT);
                },
            );
        }
        println!(
            "edgerun-frontend scene rects={} text_quads={}",
            scene.rects().len(),
            scene.text_quads().len(),
        );
        return Ok(());
    }

    instantiate_sdl_app(
        SdlGlWindowOptions::new("edgerun-frontend", 1120, 720, palette::BG)
            .min_size(360, 320)
            .frames(args.frames),
        FrontendApp::new(args.scheme),
    )
}

struct FrontendApp {
    scheme: UiColorScheme,
    shell: UiShellState,
    workspace: UiWorkspace,
}

impl FrontendApp {
    fn new(scheme: UiColorScheme) -> Self {
        let workspace = UiWorkspace::single(UiWorkspaceSurface::new(1, "Dashboard"));
        Self {
            scheme,
            shell: UiShellState::default(),
            workspace,
        }
    }
}

impl UiSurfaceApp for FrontendApp {
    fn surface(&mut self, viewport: UiRect) -> UiNode {
        column("bg-bg h-full w-full").child(text("EdgeRun Frontend"))
    }

    fn handle_action(&mut self, action: UiAction) -> UiAppControl {
        match action {
            UiAction::Hovered(_) | UiAction::Focused(_) | UiAction::Activated(_) => {
                UiAppControl::dirty()
            }
            _ => UiAppControl::clean(),
        }
    }

    fn tick(&mut self, _delta_ms: u32) -> UiAppControl {
        UiAppControl::clean()
    }
}

#[derive(Default)]
struct Args {
    frames: Option<u32>,
    dump_scene: bool,
    scheme: UiColorScheme,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = env::args().skip(1);
        let mut parsed = Self::default();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--frames" => {
                    let value = args
                        .next()
                        .ok_or("--frames requires a frame count")?
                        .parse::<u32>()
                        .map_err(|error| format!("invalid --frames value: {error}"))?;
                    parsed.frames = Some(value);
                }
                "--dump-scene" => parsed.dump_scene = true,
                "--scheme" => {
                    let value =
                        args.next().ok_or("--scheme requires dark, light, or terminal")?;
                    parsed.scheme = parse_scheme(&value)?;
                }
                "--help" | "-h" => {
                    println!(
                        "Usage: edgerun-frontend [--frames N] [--dump-scene] [--scheme dark|light|terminal]"
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(parsed)
    }
}

fn parse_scheme(value: &str) -> Result<UiColorScheme, String> {
    match value {
        "dark" => Ok(UiColorScheme::Dark),
        "light" => Ok(UiColorScheme::Light),
        "terminal" => Ok(UiColorScheme::Terminal),
        _ => Err(format!("invalid --scheme value: {value}")),
    }
}
