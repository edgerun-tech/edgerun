use lifegraph_linux_cec::{discover_cec_adapters, LinuxCecAdapter};

fn usage() {
    eprintln!("usage:");
    eprintln!("  linux-cec-tool list");
    eprintln!("  linux-cec-tool wake <instance-id>");
    eprintln!("  linux-cec-tool standby <instance-id>");
    eprintln!("  linux-cec-tool targets");
    eprintln!("  linux-cec-tool wake-gpu <gpu-instance-id> <connector-name>");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|value| value.as_str()) {
        Some("list") => match discover_cec_adapters() {
            Ok(adapters) => {
                for adapter in adapters {
                    println!(
                        "instance={} name={} driver={} devnode={} phys={:#06x?} log_mask={:#06x?} can_tx={} drm_card={} drm_connector={}",
                        adapter.instance_id,
                        adapter.adapter_name,
                        adapter.driver_name.as_deref().unwrap_or("n/a"),
                        adapter.device_node.as_deref().unwrap_or("n/a"),
                        adapter.physical_address,
                        adapter.logical_address_mask,
                        adapter.capabilities.can_transmit,
                        adapter
                            .drm_connector
                            .map(|connector| connector.card_no.to_string())
                            .unwrap_or_else(|| "n/a".into()),
                        adapter
                            .drm_connector
                            .map(|connector| connector.connector_id.to_string())
                            .unwrap_or_else(|| "n/a".into()),
                    );
                }
            }
            Err(err) => {
                eprintln!("list error: {err}");
                std::process::exit(1);
            }
        },
        Some("wake") => run_action(&args, |adapter| adapter.wake_display()),
        Some("standby") => run_action(&args, |adapter| adapter.standby_display()),
        Some("targets") => match lifegraph_linux_cec::discover_gpu_cec_targets() {
            Ok(targets) => {
                for target in targets {
                    println!(
                        "gpu={} connector={} connector_id={:?} cec_adapter={}",
                        target.gpu_instance_id,
                        target.connector_name,
                        target.connector_id,
                        target.cec_adapter
                    );
                }
            }
            Err(err) => {
                eprintln!("targets error: {err}");
                std::process::exit(1);
            }
        },
        Some("wake-gpu") => {
            let Some(gpu_instance_id) = args.get(2) else {
                usage();
                std::process::exit(2);
            };
            let Some(connector_name) = args.get(3) else {
                usage();
                std::process::exit(2);
            };
            lifegraph_linux_cec::wake_gpu_connector(gpu_instance_id, connector_name)
                .unwrap_or_else(|err| {
                    eprintln!("wake-gpu error: {err}");
                    std::process::exit(1);
                });
            println!("ok");
        }
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}

fn run_action<F>(args: &[String], action: F)
where
    F: FnOnce(&mut LinuxCecAdapter) -> Result<(), lifegraph_capabilities::CapabilityError>,
{
    let Some(instance_id) = args.get(2) else {
        usage();
        std::process::exit(2);
    };
    let adapters = discover_cec_adapters().unwrap_or_else(|err| {
        eprintln!("discover error: {err}");
        std::process::exit(1);
    });
    let info = adapters
        .into_iter()
        .find(|adapter| adapter.instance_id == *instance_id)
        .unwrap_or_else(|| {
            eprintln!("adapter not found: {instance_id}");
            std::process::exit(1);
        });
    let mut adapter = LinuxCecAdapter::open(info).unwrap_or_else(|err| {
        eprintln!("open error: {err}");
        std::process::exit(1);
    });
    action(&mut adapter).unwrap_or_else(|err| {
        eprintln!("cec action error: {err}");
        std::process::exit(1);
    });
    println!("ok");
}
