use edgerun_rt::Runtime;
use edgerun_tuya::TuyaDiscovery;
use std::env;
use std::net::SocketAddr;
use std::process::exit;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    let bind_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:0".to_string())
        .parse::<SocketAddr>()
        .unwrap_or_else(|error| {
            eprintln!("Invalid bind address: {error}");
            eprintln!("Usage: tuya-discover [bind-addr:port]");
            exit(2);
        });

    println!("Scanning for Tuya devices from {}...", bind_addr);

    let discovery = TuyaDiscovery::new();
    rt.block_on(async move {
        match discovery.broadcast_discovery(bind_addr).await {
            Ok(devices) if !devices.is_empty() => {
                println!("Found {} device(s):", devices.len());
                for dev in devices {
                    println!("  - {} @ {} (type: {:?})", dev.id, dev.ip, dev.product_type);
                }
            }
            Ok(_) => {
                println!("No devices found. Make sure devices are on same network.");
                exit(1);
            }
            Err(e) => {
                eprintln!("Discovery failed: {}", e);
                exit(1);
            }
        }
    });
}
