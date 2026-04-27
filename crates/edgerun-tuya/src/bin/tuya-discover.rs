use edgerun_rt::Runtime;
use edgerun_tuya::TuyaDiscovery;
use std::net::SocketAddr;
use std::process::exit;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    let bind_addr: SocketAddr = if cfg!(target_os = "linux") {
        "192.168.1.86:0".parse().unwrap()
    } else {
        "0.0.0.0:0".parse().unwrap()
    };

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
