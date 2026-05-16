fn main() {
    if let Err(error) = edgerun_node::ui_work_builder::run_work_builder() {
        eprintln!("edgerun-work-builder: {error}");
        std::process::exit(1);
    }
}
