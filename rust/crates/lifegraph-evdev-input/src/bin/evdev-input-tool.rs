use lifegraph_evdev_input::{discover_evdev_devices, EvdevInputBackend};
use lifegraph_input::InputDevice;

fn usage() {
    eprintln!("usage: evdev-input-tool list | read <eventN> [max_events]");
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("list") => match discover_evdev_devices() {
            Ok(devices) => {
                for device in devices {
                    println!(
                        "event={} kind={:?} name={} phys={} uniq={}",
                        device.event_node,
                        device.kind,
                        device.device_name,
                        device.physical_path.as_deref().unwrap_or(""),
                        device.unique_id.as_deref().unwrap_or(""),
                    );
                }
            }
            Err(err) => {
                eprintln!("list error: {err}");
                std::process::exit(1);
            }
        },
        Some("read") => {
            let Some(event_node) = args.next() else {
                usage();
                std::process::exit(2);
            };
            let max_events = args
                .next()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(16);
            let Some(info) = discover_evdev_devices()
                .ok()
                .and_then(|devices| devices.into_iter().find(|d| d.event_node == event_node))
            else {
                eprintln!("event node not found");
                std::process::exit(1);
            };
            let mut backend = match EvdevInputBackend::open(info) {
                Ok(backend) => backend,
                Err(err) => {
                    eprintln!("open error: {err}");
                    std::process::exit(1);
                }
            };
            match backend.read_events(max_events) {
                Ok(events) => {
                    for event in events {
                        println!(
                            "sec={} usec={} kind={:?} code={} value={}",
                            event.timestamp_sec,
                            event.timestamp_usec,
                            event.kind,
                            event.code,
                            event.value,
                        );
                    }
                }
                Err(err) => {
                    eprintln!("read error: {err}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}
