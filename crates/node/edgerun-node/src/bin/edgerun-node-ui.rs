fn main() {
    let args = parse_args();
    if let Err(error) =
        edgerun_node::ui_sdl::run_window_for_frames_with_root(args.frames, args.config)
    {
        eprintln!("edgerun-node-ui: {error}");
        std::process::exit(1);
    }
}

struct Args {
    frames: Option<u32>,
    config: std::path::PathBuf,
}

fn parse_args() -> Args {
    let mut frames = None;
    let mut config = std::env::var_os("EDGERUN_NODE_UI_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join(".edgerun").join("node-ui"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("node-ui-data"));
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--frames" {
            frames = args.next().and_then(|value| value.parse().ok());
        } else if arg == "--config" {
            if let Some(value) = args.next() {
                config = std::path::PathBuf::from(value);
            }
        }
    }
    Args { frames, config }
}
