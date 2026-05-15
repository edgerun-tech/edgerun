use edgerun_ui_core::gpu::sdl::{SdlGlWindowOptions, run_edgerun_shell_sdl_window};
use edgerun_ui_core::gpu::{
    FontAtlas, GpuScene, UiColorScheme, UiHostSession, UiShellSurfacePreset, palette,
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
        let mut session = UiHostSession::new(args.surface);
        session.set_color_scheme(args.scheme);
        let atlas = FontAtlas::load_inter(18.0)?;
        let mut scene = GpuScene::new(palette::BG);
        session.build_combined_frame(&atlas, 1120.0, 720.0);
        scene.clone_from(&session.scene);
        println!(
            "edgerun-frontend scene rects={} text_quads={}",
            scene.rects().len(),
            scene.text_quads().len()
        );
        return Ok(());
    }

    let mut session = UiHostSession::new(args.surface);
    session.set_color_scheme(args.scheme);
    let options = SdlGlWindowOptions::new(args.surface.title(), 1120, 720, palette::BG)
        .min_size(360, 320)
        .frames(args.frames);
    run_edgerun_shell_sdl_window(options, session)
}

#[derive(Default)]
struct Args {
    frames: Option<u32>,
    dump_scene: bool,
    scheme: UiColorScheme,
    surface: UiShellSurfacePreset,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = std::env::args().skip(1);
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
                    let value = args
                        .next()
                        .ok_or("--scheme requires dark, light, or terminal")?;
                    parsed.scheme = parse_scheme(&value)?;
                }
                "--surface" => {
                    let value = args.next().ok_or(
                        "--surface requires codex, workspace, lock, capability, or gallery",
                    )?;
                    parsed.surface = UiShellSurfacePreset::parse(&value)
                        .ok_or_else(|| format!("invalid --surface value: {value}"))?;
                }
                "--help" | "-h" => {
                    println!(
                        "Usage: edgerun-frontend [--frames N] [--dump-scene] [--scheme dark|light|terminal] [--surface codex|workspace|lock|capability|gallery]"
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
