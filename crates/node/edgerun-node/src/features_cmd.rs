use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn cmd_features() {
    println!("{}", feature_report_json());
}

pub fn cmd_self_test() {
    match run_self_test() {
        Ok(report) => {
            println!("{report}");
        }
        Err(error) => {
            eprintln!("self-test failed: {error}");
            std::process::exit(1);
        }
    }
}

pub fn cmd_self_test_child() {
    match self_test_child() {
        Ok(report) => {
            println!("{report}");
        }
        Err(error) => {
            eprintln!("self-test child failed: {error}");
            std::process::exit(1);
        }
    }
}

fn run_self_test() -> Result<String, String> {
    let current = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let current_meta = fs::metadata(&current).map_err(|e| format!("metadata current exe: {e}"))?;
    if current_meta.len() == 0 {
        return Err("current executable is empty".into());
    }

    let copied = copied_exe_path()?;
    if copied.exists() {
        let _ = fs::remove_file(&copied);
    }
    fs::copy(&current, &copied)
        .map_err(|e| format!("copy {} -> {}: {e}", current.display(), copied.display()))?;

    let copied_meta = fs::metadata(&copied).map_err(|e| format!("metadata copied exe: {e}"))?;
    if copied_meta.len() != current_meta.len() {
        return Err(format!(
            "copied executable size mismatch: source={} copied={}",
            current_meta.len(),
            copied_meta.len()
        ));
    }

    let features = run_child(&copied, "features")?;
    if !features.contains("\"package\":\"edgerun-node\"") || !features.contains("\"features\"") {
        return Err("copied executable feature report did not look valid".into());
    }

    let child = run_child(&copied, "self-test-child")?;
    if !child.contains("\"self_test_child\":\"ok\"") {
        return Err("copied executable child self-test did not report ok".into());
    }

    let network = run_network_self_test()?;
    let hardware = hardware_self_test()?;
    let identity = identity_self_test()?;
    let _ = fs::remove_file(&copied);

    Ok(format!(
        "{{\"self_test\":\"ok\",\"source\":\"{}\",\"copy\":\"{}\",\"bytes\":{},\"copied_features\":{},\"child\":{},\"network\":{},\"hardware\":{},\"identity\":{}}}",
        escape_json(&current.display().to_string()),
        escape_json(&copied.display().to_string()),
        current_meta.len(),
        features.trim(),
        child.trim(),
        network,
        hardware,
        identity
    ))
}

fn self_test_child() -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let meta = fs::metadata(&exe).map_err(|e| format!("metadata current exe: {e}"))?;
    if meta.len() == 0 {
        return Err("copied executable is empty".into());
    }
    Ok(format!(
        "{{\"self_test_child\":\"ok\",\"exe\":\"{}\",\"bytes\":{},\"features\":{}}}",
        escape_json(&exe.display().to_string()),
        meta.len(),
        feature_report_json()
    ))
}

fn copied_exe_path() -> Result<PathBuf, String> {
    let mut path = env::temp_dir();
    let pid = std::process::id();
    let suffix = env::consts::EXE_SUFFIX;
    path.push(format!("edgerund-self-test-{pid}{suffix}"));
    Ok(path)
}

fn run_child(exe: &PathBuf, arg: &str) -> Result<String, String> {
    let output = Command::new(exe)
        .arg(arg)
        .output()
        .map_err(|e| format!("run {} {arg}: {e}", exe.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} {arg} exited with {}; stderr={}",
            exe.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("child stdout was not utf8: {e}"))
}

fn run_network_self_test() -> Result<String, String> {
    let http = http_loopback_self_test()?;
    let dns = dns_loopback_self_test()?;
    let tftp = tftp_loopback_self_test()?;
    let tls = tls_self_test()?;
    let quic = quic_self_test()?;
    let acme = acme_self_test()?;
    let compositor = compositor_self_test()?;
    let port_80 = probe_bind_port(80);
    let port_53_tcp = probe_bind_port(53);
    let port_53_udp = probe_bind_udp_port(53);

    Ok(format!(
        "{{\"http\":{},\"dns\":{},\"tftp\":{},\"tls\":{},\"quic\":{},\"acme\":{},\"compositor\":{},\"ports\":{{\"tcp_80\":\"{}\",\"tcp_53\":\"{}\",\"udp_53\":\"{}\"}}}}",
        http,
        dns,
        tftp,
        tls,
        quic,
        acme,
        compositor,
        escape_json(&port_80),
        escape_json(&port_53_tcp),
        escape_json(&port_53_udp),
    ))
}

#[cfg(feature = "compositor")]
fn compositor_self_test() -> Result<String, String> {
    let report = edgerun_compositor::capability_report();
    if !report.registry_probe_ok {
        return Err("compositor registry probe failed".into());
    }
    if report.globals == 0 {
        return Err("compositor advertised no globals".into());
    }
    if report.wire_probe_bytes == 0 {
        return Err("compositor wire probe was empty".into());
    }

    Ok(format!(
        "{{\"status\":\"ok\",\"globals\":{},\"wl_compositor_version\":{},\"wire_probe_bytes\":{}}}",
        report.globals, report.wl_compositor_version, report.wire_probe_bytes
    ))
}

#[cfg(not(feature = "compositor"))]
fn compositor_self_test() -> Result<String, String> {
    Ok("{\"status\":\"not_compiled\"}".to_string())
}

#[cfg(feature = "all-hardware")]
fn hardware_self_test() -> Result<String, String> {
    let inventory = crate::hardware::HardwareInventory::discover();
    Ok(format!(
        concat!(
            "{{",
            "\"status\":\"ok\",",
            "\"platform\":\"{}\",",
            "\"gpus\":{},",
            "\"displays\":{},",
            "\"input\":{},",
            "\"audio_input\":{},",
            "\"audio_output\":{},",
            "\"cameras\":{},",
            "\"fingerprint\":{},",
            "\"bluetooth\":{},",
            "\"wifi\":{},",
            "\"usb\":{},",
            "\"pci\":{},",
            "\"nfc\":{},",
            "\"npu\":{},",
            "\"power\":{},",
            "\"cec\":{},",
            "\"keystore\":{}",
            "}}"
        ),
        escape_json(inventory.platform),
        inventory.gpus.len(),
        inventory.displays.len(),
        inventory.input_devices.len(),
        inventory.audio_input.len(),
        inventory.audio_output.len(),
        inventory.camera.len(),
        inventory.fingerprint_readers.len(),
        inventory.bluetooth_controllers.len(),
        inventory.wifi_interfaces.len(),
        inventory.usb_devices.len(),
        inventory.pci_devices.len(),
        inventory.nfc_adapters.len(),
        inventory.npu_devices.len(),
        inventory.power_supplies.len(),
        inventory.cec_adapters.len(),
        inventory.keystore.len()
    ))
}

#[cfg(not(feature = "all-hardware"))]
fn hardware_self_test() -> Result<String, String> {
    Ok("{\"status\":\"not_compiled\"}".to_string())
}

fn identity_self_test() -> Result<String, String> {
    let android_keystore = android_keystore_self_test()?;
    Ok(format!("{{\"android_keystore\":{android_keystore}}}"))
}

#[cfg(feature = "android-keystore")]
fn android_keystore_self_test() -> Result<String, String> {
    let input = edgerun_android_keystore::signature_input_for_record(
        "edgerun:v0:sig:self-test",
        &[9u8; 32],
    );
    if input.is_empty() {
        return Err("android keystore signature input was empty".into());
    }

    Ok(format!(
        "{{\"status\":\"ok\",\"signature_input_bytes\":{},\"provider\":\"trait-boundary\"}}",
        input.len()
    ))
}

#[cfg(not(feature = "android-keystore"))]
fn android_keystore_self_test() -> Result<String, String> {
    Ok("{\"status\":\"not_compiled\"}".to_string())
}

#[cfg(feature = "acme")]
fn acme_self_test() -> Result<String, String> {
    let account = edgerun_acme::AccountKey::generate();
    let challenge =
        edgerun_acme::DnsChallenge::new("selftest.edgerun.local", "edgerund-acme-token", &account);
    if challenge.record_name() != "_acme-challenge.selftest.edgerun.local" {
        return Err(format!(
            "unexpected ACME DNS-01 record name: {}",
            challenge.record_name()
        ));
    }
    if challenge.record_value().is_empty() {
        return Err("ACME DNS-01 record value was empty".into());
    }

    Ok(format!(
        "{{\"status\":\"ok\",\"challenge\":\"dns-01\",\"record_name\":\"{}\",\"record_value_bytes\":{}}}",
        escape_json(challenge.record_name()),
        challenge.record_value().len()
    ))
}

#[cfg(not(feature = "acme"))]
fn acme_self_test() -> Result<String, String> {
    Ok("{\"status\":\"not_compiled\"}".to_string())
}

#[cfg(feature = "tftp")]
fn tftp_loopback_self_test() -> Result<String, String> {
    let socket = UdpSocket::bind(("127.0.0.1", 0)).map_err(|e| format!("tftp bind: {e}"))?;
    let addr = socket
        .local_addr()
        .map_err(|e| format!("tftp local_addr: {e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("tftp set server timeout: {e}"))?;
    let server = socket
        .try_clone()
        .map_err(|e| format!("tftp clone socket: {e}"))?;

    let handle = thread::spawn(move || -> Result<(), String> {
        let mut buf = [0u8; 512];
        let (n, peer) = server
            .recv_from(&mut buf)
            .map_err(|e| format!("tftp recv rrq: {e}"))?;
        validate_tftp_rrq(&buf[..n], "self-test.txt")?;

        let payload = b"edgerund tftp self-test ok";
        let mut data = Vec::new();
        data.extend_from_slice(&edgerun_protocols::tftp::OP_DATA.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(payload);
        server
            .send_to(&data, peer)
            .map_err(|e| format!("tftp send data: {e}"))?;

        let (n, _) = server
            .recv_from(&mut buf)
            .map_err(|e| format!("tftp recv ack: {e}"))?;
        validate_tftp_ack(&buf[..n], 1)
    });

    let client = UdpSocket::bind(("127.0.0.1", 0)).map_err(|e| format!("tftp client bind: {e}"))?;
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("tftp client set timeout: {e}"))?;
    let rrq = build_tftp_rrq("self-test.txt", "octet");
    client
        .send_to(&rrq, addr)
        .map_err(|e| format!("tftp client send rrq: {e}"))?;

    let mut response = [0u8; 512];
    let (n, peer) = client
        .recv_from(&mut response)
        .map_err(|e| format!("tftp client recv data: {e}"))?;
    let data = parse_tftp_data(&response[..n], 1)?;
    if data != b"edgerund tftp self-test ok" {
        return Err(format!(
            "unexpected TFTP data: {}",
            String::from_utf8_lossy(data)
        ));
    }
    let ack = [
        (edgerun_protocols::tftp::OP_ACK >> 8) as u8,
        edgerun_protocols::tftp::OP_ACK as u8,
        0,
        1,
    ];
    client
        .send_to(&ack, peer)
        .map_err(|e| format!("tftp client send ack: {e}"))?;
    handle
        .join()
        .map_err(|_| "tftp server thread panicked".to_string())??;

    Ok(format!(
        "{{\"status\":\"ok\",\"addr\":\"{}\",\"file\":\"self-test.txt\",\"bytes\":{}}}",
        escape_json(&addr.to_string()),
        data.len()
    ))
}

#[cfg(not(feature = "tftp"))]
fn tftp_loopback_self_test() -> Result<String, String> {
    Ok("{\"status\":\"not_compiled\"}".to_string())
}

#[cfg(feature = "tls")]
fn tls_self_test() -> Result<String, String> {
    let cert = edgerun_tls::generate_self_signed(&["localhost", "127.0.0.1"])
        .map_err(|e| format!("tls self-signed cert: {e}"))?;
    let parsed = edgerun_tls::certificate::Certificate::from_der(&cert.cert_der)
        .map_err(|e| format!("tls parse generated cert: {e}"))?;
    if cert.cert_der.len() < 100 {
        return Err("generated TLS certificate was unexpectedly small".into());
    }
    if !parsed.is_valid_now() {
        return Err("generated TLS certificate is not currently valid".into());
    }
    Ok(format!(
        "{{\"status\":\"ok\",\"certificate_der_bytes\":{},\"certificate_chain_len\":{}}}",
        cert.cert_der.len(),
        cert.cert_chain_der.len()
    ))
}

#[cfg(not(feature = "tls"))]
fn tls_self_test() -> Result<String, String> {
    Ok("{\"status\":\"not_compiled\"}".to_string())
}

#[cfg(feature = "quic")]
fn quic_self_test() -> Result<String, String> {
    use edgerun_protocols::quic::crypto::PacketProtection;
    use edgerun_protocols::quic::handshake::QuicTlsHandshaker;
    use edgerun_protocols::quic::server_handshake::{CertificateAndKey, QuicTlsServerHandshaker};

    let cert = edgerun_tls::certificate_gen::generate_self_signed(&["localhost"])
        .map_err(|e| format!("quic tls cert: {e}"))?;
    let cert_and_key = CertificateAndKey::from(cert);
    let mut server = QuicTlsServerHandshaker::new(cert_and_key);
    let mut client = QuicTlsHandshaker::new("localhost");
    client.allow_unverified_certificates(true);

    let client_hello = client.initial_crypto_data();
    let server_hello = server
        .process_client_hello(client_hello)
        .map_err(|e| format!("quic process client hello: {e}"))?;
    client
        .process_initial_crypto(&server_hello)
        .map_err(|e| format!("quic process server hello: {e}"))?;
    let (server_handshake_data, expected_client_verify) = server
        .build_encrypted_handshake()
        .map_err(|e| format!("quic build encrypted handshake: {e}"))?;
    let client_finished = client
        .process_handshake_crypto(&server_handshake_data)
        .map_err(|e| format!("quic process handshake crypto: {e}"))?;
    let client_verify_data = client_finished
        .get(4..)
        .ok_or_else(|| "quic client finished too short".to_string())?;
    server
        .verify_client_finished(client_verify_data, &expected_client_verify)
        .map_err(|e| format!("quic verify client finished: {e}"))?;

    let mut client_transcript = client.transcript().to_vec();
    client_transcript.extend_from_slice(&client_finished);
    let client_result = client
        .build_result(&[0x01, 0x02, 0x03, 0x04], &client_transcript)
        .map_err(|e| format!("quic build client result: {e}"))?;
    let mut server_transcript = server.transcript().to_vec();
    server_transcript.extend_from_slice(&client_finished);
    let server_result = server
        .build_result(&[0x01, 0x02, 0x03, 0x04], &server_transcript)
        .map_err(|e| format!("quic build server result: {e}"))?;

    let header = b"\x40\x00\x00\x00\x01";
    let plaintext = b"edgerund quic self-test ok";
    let mut client_protection = PacketProtection::new(&client_result.app_keys);
    let mut server_protection = PacketProtection::new(&server_result.app_keys);
    let encrypted = client_protection
        .protect(header, plaintext)
        .map_err(|e| format!("quic protect app data: {e}"))?;
    let decrypted = server_protection
        .unprotect(header, 0, &encrypted)
        .map_err(|e| format!("quic unprotect app data: {e}"))?;
    if decrypted != plaintext {
        return Err("quic protected payload mismatch".into());
    }

    let udp = UdpSocket::bind(("127.0.0.1", 0)).map_err(|e| format!("quic udp bind: {e}"))?;
    let addr = udp
        .local_addr()
        .map_err(|e| format!("quic udp local_addr: {e}"))?;

    Ok(format!(
        "{{\"status\":\"ok\",\"udp_addr\":\"{}\",\"app_ciphertext_bytes\":{},\"cipher_suite\":\"TLS_AES_128_GCM_SHA256\"}}",
        escape_json(&addr.to_string()),
        encrypted.len()
    ))
}

#[cfg(not(feature = "quic"))]
fn quic_self_test() -> Result<String, String> {
    Ok("{\"status\":\"not_compiled\"}".to_string())
}

#[cfg(feature = "tftp")]
fn build_tftp_rrq(filename: &str, mode: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&edgerun_protocols::tftp::OP_RRQ.to_be_bytes());
    out.extend_from_slice(filename.as_bytes());
    out.push(0);
    out.extend_from_slice(mode.as_bytes());
    out.push(0);
    out
}

#[cfg(feature = "tftp")]
fn validate_tftp_rrq(packet: &[u8], expected_filename: &str) -> Result<(), String> {
    if packet.len() < 4 {
        return Err("TFTP RRQ too short".into());
    }
    let opcode = u16::from_be_bytes([packet[0], packet[1]]);
    if opcode != edgerun_protocols::tftp::OP_RRQ {
        return Err(format!("unexpected TFTP opcode: {opcode}"));
    }
    let rest = &packet[2..];
    let Some(filename_end) = rest.iter().position(|b| *b == 0) else {
        return Err("TFTP RRQ missing filename terminator".into());
    };
    let filename = core::str::from_utf8(&rest[..filename_end])
        .map_err(|e| format!("TFTP RRQ filename utf8: {e}"))?;
    if filename != expected_filename {
        return Err(format!("unexpected TFTP filename: {filename}"));
    }
    Ok(())
}

#[cfg(feature = "tftp")]
fn validate_tftp_ack(packet: &[u8], expected_block: u16) -> Result<(), String> {
    if packet.len() != 4 {
        return Err(format!("TFTP ACK wrong size: {}", packet.len()));
    }
    let opcode = u16::from_be_bytes([packet[0], packet[1]]);
    let block = u16::from_be_bytes([packet[2], packet[3]]);
    if opcode != edgerun_protocols::tftp::OP_ACK || block != expected_block {
        return Err(format!("unexpected TFTP ACK opcode={opcode} block={block}"));
    }
    Ok(())
}

#[cfg(feature = "tftp")]
fn parse_tftp_data(packet: &[u8], expected_block: u16) -> Result<&[u8], String> {
    if packet.len() < 4 {
        return Err("TFTP DATA too short".into());
    }
    let opcode = u16::from_be_bytes([packet[0], packet[1]]);
    let block = u16::from_be_bytes([packet[2], packet[3]]);
    if opcode != edgerun_protocols::tftp::OP_DATA || block != expected_block {
        return Err(format!(
            "unexpected TFTP DATA opcode={opcode} block={block}"
        ));
    }
    Ok(&packet[4..])
}

fn http_loopback_self_test() -> Result<String, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("http bind: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("http local_addr: {e}"))?;
    listener
        .set_nonblocking(false)
        .map_err(|e| format!("http blocking mode: {e}"))?;

    let handle = thread::spawn(move || -> Result<(), String> {
        let (mut stream, _) = listener.accept().map_err(|e| format!("http accept: {e}"))?;
        let mut buf = [0u8; 1024];
        let n = stream
            .read(&mut buf)
            .map_err(|e| format!("http read: {e}"))?;
        let request = String::from_utf8_lossy(&buf[..n]);
        if !request.starts_with("GET /self-test HTTP/1.1") {
            return Err(format!("unexpected HTTP request line: {request:?}"));
        }
        let body = "<!doctype html><title>edgerund self-test</title><h1>edgerund self-test ok</h1>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .map_err(|e| format!("http write: {e}"))?;
        Ok(())
    });

    let mut stream = TcpStream::connect(addr).map_err(|e| format!("http connect {addr}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("http set timeout: {e}"))?;
    stream
        .write_all(b"GET /self-test HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .map_err(|e| format!("http client write: {e}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("http client read: {e}"))?;
    handle
        .join()
        .map_err(|_| "http server thread panicked".to_string())??;

    if !response.starts_with("HTTP/1.1 200 OK")
        || !response.contains("edgerund self-test ok")
        || !response.contains("Content-Type: text/html")
    {
        return Err(format!("unexpected HTTP response: {response:?}"));
    }

    Ok(format!(
        "{{\"status\":\"ok\",\"addr\":\"{}\",\"served_page\":true}}",
        escape_json(&addr.to_string())
    ))
}

fn dns_loopback_self_test() -> Result<String, String> {
    let socket = UdpSocket::bind(("127.0.0.1", 0)).map_err(|e| format!("dns bind: {e}"))?;
    let addr = socket
        .local_addr()
        .map_err(|e| format!("dns local_addr: {e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("dns set server timeout: {e}"))?;
    let server = socket
        .try_clone()
        .map_err(|e| format!("dns clone socket: {e}"))?;

    let handle = thread::spawn(move || -> Result<(), String> {
        let mut buf = [0u8; 512];
        let (n, peer) = server
            .recv_from(&mut buf)
            .map_err(|e| format!("dns recv: {e}"))?;
        let response = build_dns_response(&buf[..n])?;
        server
            .send_to(&response, peer)
            .map_err(|e| format!("dns send: {e}"))?;
        Ok(())
    });

    let client = UdpSocket::bind(("127.0.0.1", 0)).map_err(|e| format!("dns client bind: {e}"))?;
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("dns client set timeout: {e}"))?;
    let query = build_dns_query(0xedd9, "selftest.edgerun.local")?;
    client
        .send_to(&query, addr)
        .map_err(|e| format!("dns client send: {e}"))?;
    let mut response = [0u8; 512];
    let (n, _) = client
        .recv_from(&mut response)
        .map_err(|e| format!("dns client recv: {e}"))?;
    handle
        .join()
        .map_err(|_| "dns server thread panicked".to_string())??;

    let answer = parse_dns_a_answer(&response[..n], 0xedd9)?;
    if answer != Ipv4Addr::new(127, 0, 0, 1) {
        return Err(format!("unexpected DNS A answer: {answer}"));
    }

    Ok(format!(
        "{{\"status\":\"ok\",\"addr\":\"{}\",\"query\":\"selftest.edgerun.local\",\"answer\":\"{}\"}}",
        escape_json(&addr.to_string()),
        answer
    ))
}

fn build_dns_query(id: u16, name: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&0x0100u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    encode_dns_name(name, &mut out)?;
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    Ok(out)
}

fn build_dns_response(query: &[u8]) -> Result<Vec<u8>, String> {
    if query.len() < 12 {
        return Err("DNS query too short".into());
    }
    let question_end = dns_question_end(query)?;
    let mut out = Vec::new();
    out.extend_from_slice(&query[0..2]);
    out.extend_from_slice(&0x8180u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&query[12..question_end]);
    out.extend_from_slice(&0xc00cu16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&60u32.to_be_bytes());
    out.extend_from_slice(&4u16.to_be_bytes());
    out.extend_from_slice(&[127, 0, 0, 1]);
    Ok(out)
}

fn parse_dns_a_answer(response: &[u8], expected_id: u16) -> Result<Ipv4Addr, String> {
    if response.len() < 12 {
        return Err("DNS response too short".into());
    }
    let id = u16::from_be_bytes([response[0], response[1]]);
    if id != expected_id {
        return Err(format!("DNS id mismatch: {id:#x}"));
    }
    let answer_count = u16::from_be_bytes([response[6], response[7]]);
    if answer_count == 0 {
        return Err("DNS response had no answers".into());
    }
    let mut offset = dns_question_end(response)?;
    if offset + 16 > response.len() {
        return Err("DNS answer too short".into());
    }
    if response[offset] & 0xc0 == 0xc0 {
        offset += 2;
    } else {
        offset = skip_dns_name(response, offset)?;
    }
    let rr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
    let rr_class = u16::from_be_bytes([response[offset + 2], response[offset + 3]]);
    let rd_len = u16::from_be_bytes([response[offset + 8], response[offset + 9]]) as usize;
    offset += 10;
    if rr_type != 1 || rr_class != 1 || rd_len != 4 || offset + 4 > response.len() {
        return Err("DNS answer was not an A/IN record".into());
    }
    Ok(Ipv4Addr::new(
        response[offset],
        response[offset + 1],
        response[offset + 2],
        response[offset + 3],
    ))
}

fn dns_question_end(packet: &[u8]) -> Result<usize, String> {
    let offset = skip_dns_name(packet, 12)?;
    if offset + 4 > packet.len() {
        return Err("DNS question missing qtype/qclass".into());
    }
    Ok(offset + 4)
}

fn skip_dns_name(packet: &[u8], mut offset: usize) -> Result<usize, String> {
    loop {
        let Some(&len) = packet.get(offset) else {
            return Err("DNS name outside packet".into());
        };
        offset += 1;
        if len == 0 {
            return Ok(offset);
        }
        if len & 0xc0 == 0xc0 {
            if offset >= packet.len() {
                return Err("DNS compressed name truncated".into());
            }
            return Ok(offset + 1);
        }
        offset += len as usize;
        if offset > packet.len() {
            return Err("DNS label outside packet".into());
        }
    }
}

fn encode_dns_name(name: &str, out: &mut Vec<u8>) -> Result<(), String> {
    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(format!("invalid DNS label: {label:?}"));
        }
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    Ok(())
}

fn probe_bind_port(port: u16) -> String {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => match listener.local_addr() {
            Ok(SocketAddr::V4(_)) | Ok(SocketAddr::V6(_)) => "available".to_string(),
            Err(e) => format!("bound_addr_error:{e}"),
        },
        Err(e) => bind_error_label(&e),
    }
}

fn probe_bind_udp_port(port: u16) -> String {
    match UdpSocket::bind(("127.0.0.1", port)) {
        Ok(socket) => match socket.local_addr() {
            Ok(SocketAddr::V4(_)) | Ok(SocketAddr::V6(_)) => "available".to_string(),
            Err(e) => format!("bound_addr_error:{e}"),
        },
        Err(e) => bind_error_label(&e),
    }
}

fn bind_error_label(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::AddrInUse => "in_use".to_string(),
        std::io::ErrorKind::PermissionDenied => "permission_denied".to_string(),
        _ => format!("error:{error}"),
    }
}

fn feature_report_json() -> String {
    format!(
        concat!(
            "{{",
            "\"package\":\"edgerun-node\",",
            "\"version\":\"{}\",",
            "\"target\":\"{}\",",
            "\"profile\":\"{}\",",
            "\"features\":{{",
            "\"std\":{},",
            "\"oci\":{},",
            "\"oci_registry_client\":{},",
            "\"oci_edgefs\":{},",
            "\"oci_gzip\":{},",
            "\"oci_zstd\":{},",
            "\"tls\":{},",
            "\"https\":{},",
            "\"quic\":{},",
            "\"tftp\":{},",
            "\"acme\":{},",
            "\"http\":{},",
            "\"dns\":{},",
            "\"dhcp\":{},",
            "\"smtp\":{},",
            "\"imap\":{},",
            "\"proxy\":{},",
            "\"derived_db\":{},",
            "\"android_keystore\":{},",
            "\"compositor\":{},",
            "\"display\":{},",
            "\"hardware\":{},",
            "\"all_hardware\":{}",
            "}},",
            "\"compiled_components\":[{}]",
            "}}"
        ),
        env!("CARGO_PKG_VERSION"),
        runtime_target(),
        runtime_profile(),
        cfg!(feature = "std"),
        cfg!(feature = "oci"),
        cfg!(feature = "oci"),
        cfg!(feature = "oci"),
        cfg!(feature = "oci"),
        cfg!(feature = "oci"),
        cfg!(feature = "tls"),
        cfg!(feature = "https"),
        cfg!(feature = "quic"),
        cfg!(feature = "tftp"),
        cfg!(feature = "acme"),
        cfg!(feature = "http"),
        cfg!(feature = "dns"),
        cfg!(feature = "dhcp"),
        cfg!(feature = "smtp"),
        cfg!(feature = "imap"),
        cfg!(feature = "proxy"),
        cfg!(feature = "derived-db"),
        cfg!(feature = "android-keystore"),
        cfg!(feature = "compositor"),
        display_compiled(),
        cfg!(feature = "hardware"),
        cfg!(feature = "all-hardware"),
        compiled_components_json(),
    )
}

fn runtime_target() -> String {
    format!(
        "{}-unknown-{}-{}",
        env::consts::ARCH,
        env::consts::OS,
        target_env()
    )
}

fn target_env() -> &'static str {
    if cfg!(target_env = "musl") {
        "musl"
    } else if cfg!(target_env = "gnu") {
        "gnu"
    } else if cfg!(target_env = "msvc") {
        "msvc"
    } else {
        "unknown"
    }
}

fn runtime_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn compiled_components_json() -> String {
    let mut parts = Vec::new();
    parts.push("\"node\"");
    parts.push("\"stream\"");
    parts.push("\"storage\"");
    parts.push("\"mesh\"");
    parts.push("\"exchange\"");

    if cfg!(feature = "oci") {
        parts.push("\"oci\"");
        parts.push("\"oci-registry-client\"");
        parts.push("\"oci-edgefs\"");
        parts.push("\"oci-gzip\"");
        parts.push("\"oci-zstd\"");
    }
    if cfg!(feature = "tls") {
        parts.push("\"tls\"");
    }
    if cfg!(feature = "https") {
        parts.push("\"https\"");
    }
    if cfg!(feature = "quic") {
        parts.push("\"quic\"");
    }
    if cfg!(feature = "tftp") {
        parts.push("\"tftp\"");
    }
    if cfg!(feature = "acme") {
        parts.push("\"acme\"");
        parts.push("\"acme-dns-01\"");
    }
    if cfg!(feature = "http") {
        parts.push("\"http\"");
    }
    if cfg!(feature = "dns") {
        parts.push("\"dns\"");
    }
    if cfg!(feature = "dhcp") {
        parts.push("\"dhcp\"");
    }
    if cfg!(feature = "smtp") {
        parts.push("\"smtp\"");
    }
    if cfg!(feature = "imap") {
        parts.push("\"imap\"");
    }
    if cfg!(feature = "proxy") {
        parts.push("\"proxy\"");
    }
    if cfg!(feature = "derived-db") {
        parts.push("\"derived-db\"");
    }
    if cfg!(feature = "android-keystore") {
        parts.push("\"android-keystore\"");
    }
    if compositor_compiled() {
        parts.push("\"compositor\"");
    }
    if cfg!(feature = "hardware") {
        parts.push("\"evdev-input\"");
        parts.push("\"alsa-speaker\"");
        parts.push("\"alsa-microphone\"");
        parts.push("\"v4l2-camera\"");
    }
    if cfg!(feature = "all-hardware") {
        parts.push("\"gpu\"");
        parts.push("\"fingerprint\"");
        parts.push("\"bluetooth\"");
        parts.push("\"wifi\"");
        parts.push("\"usb\"");
        parts.push("\"pci\"");
        parts.push("\"nfc\"");
        parts.push("\"npu\"");
        parts.push("\"power\"");
        parts.push("\"cec\"");
        parts.push("\"amd-xdna\"");
        parts.push("\"virtual-disk\"");
        parts.push("\"machine-report\"");
    }
    if display_compiled() {
        parts.push("\"drm-display\"");
    }

    parts.join(",")
}

#[cfg(feature = "compositor")]
fn compositor_compiled() -> bool {
    edgerun_compositor::capability_report().registry_probe_ok
}

#[cfg(not(feature = "compositor"))]
fn compositor_compiled() -> bool {
    false
}

fn display_compiled() -> bool {
    cfg!(feature = "display") || cfg!(feature = "all-hardware")
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
