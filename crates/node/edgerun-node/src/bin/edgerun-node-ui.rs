fn main() {
    let frames = parse_frames_arg();
    if let Err(error) = edgerun_node::ui_sdl::run_window_for_frames(frames) {
        eprintln!("edgerun-node-ui: {error}");
        std::process::exit(1);
    }
}

fn parse_frames_arg() -> Option<u32> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--frames" {
            return args.next().and_then(|value| value.parse().ok());
        }
    }
    None
}
