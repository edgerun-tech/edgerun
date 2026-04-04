use lifegraph_linux_npu::discover_linux_npus;
use lifegraph_npu::NpuDevice;

fn usage() {
    eprintln!("usage: linux-npu-tool list");
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("list") => match discover_linux_npus() {
            Ok(devices) => {
                for info in devices {
                    let backend = lifegraph_linux_npu::LinuxNpuBackend { info };
                    match backend.npu_info() {
                        Ok(npu) => println!(
                            "instance={} name={} driver={} pci={} vendor={:#06x?} device={:#06x?} char_dev={} submission={} firmware={}",
                            npu.instance_id,
                            npu.display_name,
                            npu.driver_name.as_deref().unwrap_or("n/a"),
                            npu.pci_address.as_deref().unwrap_or("n/a"),
                            npu.vendor_id,
                            npu.device_id,
                            npu.character_device.as_deref().unwrap_or("n/a"),
                            npu.supports_submission,
                            npu.firmware_version.as_deref().unwrap_or("n/a"),
                        ),
                        Err(err) => {
                            eprintln!("npu info error: {err}");
                            std::process::exit(1);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("list error: {err}");
                std::process::exit(1);
            }
        },
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}
