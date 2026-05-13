use std::path::Path;

fn main() {
    let candidates = [
        "/usr/share/fonts/inter/InterVariable.ttf",
        "/usr/share/fonts/inter/Inter.ttc",
        "/usr/share/fonts/Inter/InterVariable.ttf",
        "/usr/share/fonts/Inter/Inter.ttc",
    ];
    for path in candidates {
        if Path::new(path).exists() {
            println!("cargo:rustc-env=CODEX_GL_INTER_FONT={path}");
            return;
        }
    }
    panic!("Inter font not found; install Inter or set up a local font asset");
}
