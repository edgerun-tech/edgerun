use edgerun_linux_nfc::discover_nfc_adapters;

fn main() {
    match discover_nfc_adapters() {
        Ok(list) => {
            for adapter in list {
                println!(
                    "{} protocol={} power={:?}",
                    adapter.name,
                    adapter.protocol_name.unwrap_or_default(),
                    adapter.power_state
                );
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
