use edgerun_relay::Relay;
use std::env;

fn main() {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7373".to_owned());
    eprintln!("edgerun-relay listening on {addr}");
    if let Err(error) = Relay::new().serve(&addr) {
        eprintln!("edgerun-relay failed: {error}");
        std::process::exit(1);
    }
}
