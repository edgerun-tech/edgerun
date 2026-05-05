use edgerun_v4l2_camera::{
    group_camera_probes, probe_camera_devices, select_paired_camera, V4l2CameraDevice,
    V4l2CameraProbe,
};
use std::path::PathBuf;

fn print_probe(probe: &V4l2CameraProbe) {
    println!("device: {}", probe.info.devnode.display());
    println!("  card: {}", probe.info.card);
    println!("  driver: {}", probe.info.driver);
    println!("  bus: {}", probe.info.bus_info);
    println!("  version: {}", probe.info.version);
    println!(
        "  capabilities: capture={} streaming={}",
        probe.info.supports_video_capture(),
        probe.info.supports_streaming()
    );
    println!("  infrared: {:?}", probe.info.infrared);
    println!("  role: {:?}", probe.stream_role);
    println!(
        "  current_format: {:?} {}x{} stride={} image_size={}",
        probe.current_format.pixel_format,
        probe.current_format.width,
        probe.current_format.height,
        probe.current_format.stride,
        probe.current_format.image_size
    );
    if probe.available_formats.is_empty() {
        println!("  available_formats: <none>");
    } else {
        println!("  available_formats:");
        for fmt in &probe.available_formats {
            println!(
                "    - {:?} raw=0x{raw:08x} flags=0x{flags:08x} desc={desc}",
                fmt.pixel_format,
                raw = fmt.raw_pixel_format,
                flags = fmt.flags,
                desc = fmt.description
            );
        }
    }
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  v4l2-camera-tool list");
    eprintln!("  v4l2-camera-tool groups");
    eprintln!("  v4l2-camera-tool probe <device>");
    eprintln!("  v4l2-camera-tool capture <device> [timeout_ms]");
    eprintln!("  v4l2-camera-tool capture-paired [timeout_ms]");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("list") => match probe_camera_devices() {
            Ok(probes) => {
                if probes.is_empty() {
                    println!("no V4L2 camera devices found");
                }
                for probe in probes {
                    print_probe(&probe);
                }
            }
            Err(err) => {
                eprintln!("list failed: {err}");
                std::process::exit(1);
            }
        },
        Some("groups") => match probe_camera_devices() {
            Ok(probes) => {
                let groups = group_camera_probes(probes);
                if groups.is_empty() {
                    println!("no grouped V4L2 camera devices found");
                }
                if let Some(selection) = select_paired_camera(&groups) {
                    println!(
                        "selected_pair: rgb={:?} infrared={:?} depth={:?}",
                        selection
                            .rgb
                            .as_ref()
                            .map(|p| p.info.devnode.display().to_string()),
                        selection
                            .infrared
                            .as_ref()
                            .map(|p| p.info.devnode.display().to_string()),
                        selection
                            .depth
                            .as_ref()
                            .map(|p| p.info.devnode.display().to_string())
                    );
                }
                for group in groups {
                    println!("group: {}", group.key);
                    println!("  card: {}", group.card);
                    println!("  bus: {}", group.bus_info);
                    for device in &group.devices {
                        println!(
                            "  - {} role={:?} infrared={:?}",
                            device.info.devnode.display(),
                            device.stream_role,
                            device.info.infrared
                        );
                    }
                }
            }
            Err(err) => {
                eprintln!("groups failed: {err}");
                std::process::exit(1);
            }
        },
        Some("capture-paired") => match probe_camera_devices() {
            Ok(probes) => {
                let groups = group_camera_probes(probes);
                let timeout_ms = args
                    .get(2)
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(1500);
                let Some(selection) = select_paired_camera(&groups) else {
                    eprintln!("capture-paired failed: no paired camera selection available");
                    std::process::exit(1);
                };
                if let Some(rgb) = &selection.rgb {
                    let device = V4l2CameraDevice::new(rgb.info.devnode.clone());
                    match device.capture_frame(timeout_ms) {
                        Ok(frame) => println!(
                            "rgb captured {:?} {}x{} bytes={}",
                            frame.format,
                            frame.width,
                            frame.height,
                            frame.bytes.len()
                        ),
                        Err(err) => println!("rgb capture failed: {err}"),
                    }
                }
                if let Some(ir) = &selection.infrared {
                    let device = V4l2CameraDevice::new(ir.info.devnode.clone());
                    match device.capture_frame(timeout_ms) {
                        Ok(frame) => println!(
                            "infrared captured {:?} {}x{} bytes={}",
                            frame.format,
                            frame.width,
                            frame.height,
                            frame.bytes.len()
                        ),
                        Err(err) => println!("infrared capture failed: {err}"),
                    }
                }
                if let Some(depth) = &selection.depth {
                    let device = V4l2CameraDevice::new(depth.info.devnode.clone());
                    match device.capture_frame(timeout_ms) {
                        Ok(frame) => println!(
                            "depth captured {:?} {}x{} bytes={}",
                            frame.format,
                            frame.width,
                            frame.height,
                            frame.bytes.len()
                        ),
                        Err(err) => println!("depth capture failed: {err}"),
                    }
                }
            }
            Err(err) => {
                eprintln!("capture-paired failed: {err}");
                std::process::exit(1);
            }
        },
        Some("probe") => {
            let Some(device) = args.get(2) else {
                usage();
                std::process::exit(2);
            };
            let device = V4l2CameraDevice::new(PathBuf::from(device));
            match device.probe() {
                Ok(probe) => print_probe(&probe),
                Err(err) => {
                    eprintln!("probe failed: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("capture") => {
            let Some(device) = args.get(2) else {
                usage();
                std::process::exit(2);
            };
            let timeout_ms = args
                .get(3)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1500);
            let device = V4l2CameraDevice::new(PathBuf::from(device));
            match device.capture_frame(timeout_ms) {
                Ok(frame) => {
                    println!(
                        "captured {:?} {}x{} stride={} bytes={}",
                        frame.format,
                        frame.width,
                        frame.height,
                        frame.stride,
                        frame.bytes.len()
                    );
                }
                Err(err) => {
                    eprintln!("capture failed: {err}");
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
