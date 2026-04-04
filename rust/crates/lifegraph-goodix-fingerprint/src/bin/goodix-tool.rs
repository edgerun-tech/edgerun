use lifegraph_fingerprint::{FingerprintCapturePurpose, FingerprintReader};
use lifegraph_goodix_fingerprint::{
    build_goodix_package, discover_supported_devices, GoodixFingerprintReader,
};

fn usage() {
    eprintln!("usage: goodix-tool <command> [args]\n\ncommands:\n  probe\n  features\n  list\n  capture-enroll\n  capture-verify\n  verify-live\n  enroll-init <label> [samples]\n  begin-template\n  commit-template <template_id_hex> <label>\n  delete-template <template_id_hex>\n  delete-all\n  finger-mode-status\n  finger-down\n  finger-up\n  stage-default-config\n  write-default-config\n  shield-on\n  shield-off");
}

fn open_reader() -> Result<GoodixFingerprintReader, String> {
    let devices = discover_supported_devices().map_err(|e| e.to_string())?;
    let device = devices
        .into_iter()
        .next()
        .ok_or_else(|| "no supported Goodix fingerprint device found".to_string())?;
    GoodixFingerprintReader::new(device).map_err(|e| e.to_string())
}

fn main() {
    if let Err(err) = real_main() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
        return Err("missing command".into());
    };

    match cmd.as_str() {
        "probe" => {
            let reader = open_reader()?;
            let probe = reader.probe_device().map_err(|e| e.to_string())?;
            println!("transport.interface={}", probe.transport.interface_number);
            println!("transport.bulk_in={:?}", probe.transport.bulk_in_endpoint);
            println!("transport.bulk_out={:?}", probe.transport.bulk_out_endpoint);
            println!(
                "transport.interrupt_in={:?}",
                probe.transport.interrupt_in_endpoint
            );
            println!("usb.vendor_id={:04x}", probe.device_descriptor.vendor_id);
            println!("usb.product_id={:04x}", probe.device_descriptor.product_id);
            println!("usb.active_configuration={}", probe.active_configuration);
            println!(
                "usb.num_interfaces={}",
                probe.configuration_descriptor.num_interfaces
            );
            println!("usb.manufacturer={:?}", probe.strings.manufacturer);
            println!("usb.product={:?}", probe.strings.product);
            println!("usb.serial={:?}", probe.strings.serial_number);
            if let Some(version) = probe.version_info {
                println!("goodix.firmware_type={}", version.firmware_type_string());
                println!(
                    "goodix.firmware_version={}",
                    version.firmware_version_string()
                );
            } else {
                println!("goodix.version=unavailable");
            }
        }
        "features" => {
            let reader = open_reader()?;
            let f = reader.supported_features();
            println!("match_on_sensor={}", f.match_on_sensor);
            println!("persistent_templates={}", f.persistent_templates);
            println!("template_listing={}", f.template_listing);
            println!("template_delete={}", f.template_delete);
            println!("delete_all_templates={}", f.delete_all_templates);
            println!("capture={}", f.capture);
            println!("identify={}", f.identify);
            println!("enroll_init={}", f.enroll_init);
            println!("enroll_update={}", f.enroll_update);
            println!("duplicate_check={}", f.duplicate_check);
            println!("commit_enrollment={}", f.commit_enrollment);
            println!("update_config={}", f.update_config);
            println!("finger_mode_query={}", f.finger_mode_query);
            println!("finger_down_mode={}", f.finger_down_mode);
            println!("finger_up_mode={}", f.finger_up_mode);
            println!("power_button_shield={}", f.power_button_shield);
        }
        "list" => {
            let reader = open_reader()?;
            let templates = reader.list_templates().map_err(|e| e.to_string())?;
            println!("count={}", templates.len());
            for t in templates {
                println!("template_id={} label={}", t.template_id, t.label);
            }
        }
        "capture-enroll" => {
            let mut reader = open_reader()?;
            let capture = reader
                .capture(FingerprintCapturePurpose::Enrollment, 5000)
                .map_err(|e| e.to_string())?;
            println!("quality={:?}", capture.quality);
            println!("verified={}", capture.state.verified);
            println!("user_present={}", capture.state.user_present);
            println!("hardware_protected={}", capture.state.hardware_protected);
        }
        "capture-verify" => {
            let mut reader = open_reader()?;
            let capture = reader
                .capture(FingerprintCapturePurpose::Verification, 5000)
                .map_err(|e| e.to_string())?;
            println!("quality={:?}", capture.quality);
            println!("verified={}", capture.state.verified);
            println!("user_present={}", capture.state.user_present);
            println!("hardware_protected={}", capture.state.hardware_protected);
        }
        "verify-live" => {
            let reader = open_reader()?;
            let result = reader.verify_live_finger().map_err(|e| e.to_string())?;
            println!("matched={}", result.matched);
            println!("result=0x{:02x}", result.result);
            println!("reject_detail={:?}", result.reject_detail);
            println!("score={:?}", result.score);
            println!("study={:?}", result.study);
            if let Some(template) = result.template {
                println!("template_id={}", {
                    let mut s = String::new();
                    for b in template.template_id {
                        s.push_str(&format!("{b:02x}"));
                    }
                    s
                });
                println!("finger_index={}", template.finger_index);
                println!("label={}", String::from_utf8_lossy(&template.payload));
            }
        }
        "enroll-init" => {
            let label = args.next().ok_or_else(|| "missing label".to_string())?;
            let samples_required = match args.next() {
                Some(s) => s
                    .parse::<u8>()
                    .map_err(|_| "invalid samples value".to_string())?,
                None => 8,
            };
            let mut reader = open_reader()?;
            let session = reader
                .begin_enrollment(&lifegraph_fingerprint::FingerprintEnrollRequest {
                    label,
                    samples_required,
                    require_hardware_match: true,
                })
                .map_err(|e| e.to_string())?;
            println!("session_id={}", session.session_id);
            println!("samples_required={}", session.samples_required);
        }

        "begin-template" => {
            let reader = open_reader()?;
            let template = reader
                .begin_enrollment_template()
                .map_err(|e| e.to_string())?;
            match template {
                Some(t) => {
                    let mut s = String::new();
                    for b in t {
                        s.push_str(&format!("{b:02x}"));
                    }
                    println!("template_id={}", s);
                }
                None => println!("template_id=none"),
            }
        }
        "commit-template" => {
            let template_id = args
                .next()
                .ok_or_else(|| "missing template_id_hex".to_string())?;
            let label = args.next().ok_or_else(|| "missing label".to_string())?;
            let bytes = {
                if template_id.len() != 64 {
                    return Err("template_id_hex must be 64 hex chars".into());
                }
                let mut out = [0u8; 32];
                for i in 0..32 {
                    out[i] = u8::from_str_radix(&template_id[i * 2..i * 2 + 2], 16)
                        .map_err(|_| "invalid template_id hex".to_string())?;
                }
                out
            };
            let reader = open_reader()?;
            reader
                .commit_template_id(&bytes, &label)
                .map_err(|e| e.to_string())?;
            println!("ok");
        }
        "delete-template" => {
            let template_id = args
                .next()
                .ok_or_else(|| "missing template_id_hex".to_string())?;
            let mut reader = open_reader()?;
            reader
                .delete_template(&template_id)
                .map_err(|e| e.to_string())?;
            println!("ok");
        }
        "delete-all" => {
            let reader = open_reader()?;
            reader
                .delete_all_templates_live()
                .map_err(|e| e.to_string())?;
            println!("ok");
        }
        "finger-up" => {
            let reader = open_reader()?;
            let transport = reader.transport_info().map_err(|e| e.to_string())?;
            let mut usb = reader.open_claimed_transport().map_err(|e| e.to_string())?;
            let status = usb
                .set_finger_up_mode(&transport)
                .map_err(|e| e.to_string())?;
            println!("status=0x{:02x}", status.status);
        }
        "finger-down" => {
            let reader = open_reader()?;
            let status = reader
                .set_finger_down_mode_live()
                .map_err(|e| e.to_string())?;
            println!("status=0x{:02x}", status.status);
        }
        "finger-mode-status" => {
            let reader = open_reader()?;
            let status = reader.get_finger_mode_live().map_err(|e| e.to_string())?;
            println!("status=0x{:02x}", status.status);
        }
        "stage-default-config" => {
            let reader = open_reader()?;
            let cfg = reader
                .update_default_config_live(false)
                .map_err(|e| e.to_string())?;
            println!("status=0x{:02x}", cfg.status);
            println!("max_stored_prints={}", cfg.max_stored_prints);
        }
        "write-default-config" => {
            let reader = open_reader()?;
            let cfg = reader
                .update_default_config_live(true)
                .map_err(|e| e.to_string())?;
            println!("status=0x{:02x}", cfg.status);
            println!("max_stored_prints={}", cfg.max_stored_prints);
        }
        "shield-on" => {
            let reader = open_reader()?;
            reader
                .set_power_button_shield_live(true)
                .map_err(|e| e.to_string())?;
            println!("ok");
        }
        "debug-version" => {
            let reader = open_reader()?;
            let transport = reader.transport_info().map_err(|e| e.to_string())?;
            let mut usb = reader.open_claimed_transport().map_err(|e| e.to_string())?;
            let bulk_out = transport
                .bulk_out_endpoint
                .ok_or_else(|| "missing bulk-out".to_string())?;
            let bulk_in = transport
                .bulk_in_endpoint
                .ok_or_else(|| "missing bulk-in".to_string())?;
            let mut request = build_goodix_package(0xd0, 0x00, &[0]);
            let sent = usb
                .bulk_transfer(bulk_out, &mut request, 500)
                .map_err(|e| e.to_string())?;
            println!("sent={sent}");
            let mut ack_buf = vec![0u8; 2048];
            let ack_len = usb
                .bulk_transfer(bulk_in, &mut ack_buf, 500)
                .map_err(|e| e.to_string())?;
            println!("ack_len={ack_len}");
            for b in &ack_buf[..ack_len] {
                print!("{b:02x}");
            }
            println!();
            let mut data_buf = vec![0u8; 2048];
            let data_len = usb
                .bulk_transfer(bulk_in, &mut data_buf, 1000)
                .map_err(|e| e.to_string())?;
            println!("data_len={data_len}");
            for b in &data_buf[..data_len] {
                print!("{b:02x}");
            }
            println!();
        }
        "shield-off" => {
            let reader = open_reader()?;
            reader
                .set_power_button_shield_live(false)
                .map_err(|e| e.to_string())?;
            println!("ok");
        }
        _ => {
            usage();
            return Err(format!("unknown command: {cmd}"));
        }
    }

    Ok(())
}
