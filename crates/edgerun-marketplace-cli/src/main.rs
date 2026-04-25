fn main() {
    if let Err(e) = edgerun_marketplace_cli::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
