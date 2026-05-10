use edgerun_crypto::{Ed25519SigningKey, Signer};
use edgerun_protocols::keygen::EphemeralNodeIdentity;
use edgerun_relay::{
    MAX_PAYLOAD_LEN, Relay, delivery_receipt_preimage, encode_packet, read_message, report_hash,
    report_receipt_preimage, request_hash, sha256_array, submit_preimage, verify_submit,
    write_message,
};
use edgerun_wire::{
    RELAY_DELIVERY_STATUS_ACCEPTED, RELAY_REPORT_STATUS_ACCEPTED, RELAY_WIRE_ABI_VERSION, RelayAck,
    RelayDeliveryReceipt, RelayDeliveryReport, RelayDeliveryReportReceipt, RelayDeliveryRequest,
    RelayIdentity, RelayMessage, RelayRegister, RelaySignature, RelaySubmit,
    SIGNATURE_ALGORITHM_ECDSA_P256_SHA256, SIGNATURE_ALGORITHM_ED25519,
};
use std::hint::black_box;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

const E2E_BATCH_DELIVERIES: usize = 50_000;

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>().into_iter();
    let first = args.next();
    if matches!(first.as_deref(), Some("--e2e" | "e2e")) {
        let iterations = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(10_000);
        let payload_len = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .min(MAX_PAYLOAD_LEN);
        bench_e2e(iterations, payload_len);
        return;
    }
    if matches!(
        first.as_deref(),
        Some("--e2e-pipeline" | "e2e-pipeline" | "pipeline")
    ) {
        let iterations = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(10_000);
        let payload_len = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .min(MAX_PAYLOAD_LEN);
        let window = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .max(1);
        bench_e2e_pipeline(iterations, payload_len, window);
        return;
    }
    if matches!(
        first.as_deref(),
        Some("--e2e-precomputed" | "e2e-precomputed" | "precomputed")
    ) {
        let iterations = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(10_000);
        let payload_len = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .min(MAX_PAYLOAD_LEN);
        let window = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .max(1);
        bench_e2e_precomputed(iterations, payload_len, window);
        return;
    }
    if matches!(
        first.as_deref(),
        Some("--client-e2e-pipeline" | "client-e2e-pipeline" | "client-pipeline")
    ) {
        let addr = args
            .next()
            .unwrap_or_else(|| "127.0.0.1:7373".to_owned())
            .parse()
            .expect("relay socket address");
        let iterations = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(10_000);
        let payload_len = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .min(MAX_PAYLOAD_LEN);
        let window = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .max(1);
        bench_client_e2e_pipeline(addr, iterations, payload_len, window);
        return;
    }
    if matches!(
        first.as_deref(),
        Some("--client-udp-e2e" | "client-udp-e2e" | "udp-e2e")
    ) {
        let addr = args
            .next()
            .unwrap_or_else(|| "127.0.0.1:7373".to_owned())
            .parse()
            .expect("relay socket address");
        let iterations = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(1_000);
        let payload_len = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .min(MAX_PAYLOAD_LEN);
        bench_client_udp_e2e(addr, iterations, payload_len);
        return;
    }
    if matches!(
        first.as_deref(),
        Some("--client-udp-pipeline" | "client-udp-pipeline" | "udp-pipeline")
    ) {
        let addr = args
            .next()
            .unwrap_or_else(|| "127.0.0.1:7373".to_owned())
            .parse()
            .expect("relay socket address");
        let iterations = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(1_000);
        let payload_len = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .min(MAX_PAYLOAD_LEN);
        let window = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(32)
            .max(1);
        bench_client_udp_pipeline(addr, iterations, payload_len, window);
        return;
    }
    if matches!(
        first.as_deref(),
        Some("--client-custom-quic" | "client-custom-quic" | "custom-quic")
    ) {
        let addr = args
            .next()
            .unwrap_or_else(|| "127.0.0.1:7373".to_owned())
            .parse()
            .expect("relay socket address");
        let iterations = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(1_000);
        let payload_len = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(256)
            .min(MAX_PAYLOAD_LEN);
        let window = args
            .next()
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(32)
            .max(1);
        bench_client_custom_quic(addr, iterations, payload_len, window);
        return;
    }

    let iterations = first
        .and_then(|arg| arg.parse::<usize>().ok())
        .unwrap_or(100_000);
    let payload_len = args
        .next()
        .and_then(|arg| arg.parse::<usize>().ok())
        .unwrap_or(256)
        .min(MAX_PAYLOAD_LEN);

    let payload = vec![0xA5; payload_len];
    let (_, to) = ed25519_identity(2);
    let ed25519_submit = submit_ed25519(3, to.clone(), [0x11; 32], &payload);
    let p256_submit = submit_p256(4, to, [0x22; 32], &payload);
    let encoded = encode_packet(&RelayMessage::Submit(ed25519_submit.clone())).expect("encode");

    println!(
        "iterations={iterations} payload_len={payload_len} encoded_submit_len={}",
        encoded.len()
    );
    print_result(
        "encode submit",
        iterations,
        time_loop(iterations, || {
            black_box(
                encode_packet(black_box(&RelayMessage::Submit(ed25519_submit.clone()))).unwrap(),
            );
        }),
    );
    print_result(
        "decode submit",
        iterations,
        time_loop(iterations, || {
            black_box(edgerun_relay::decode_packet(black_box(&encoded)).unwrap());
        }),
    );
    print_result(
        "verify ed25519 submit",
        iterations,
        time_loop(iterations, || {
            black_box(verify_submit(black_box(&ed25519_submit)));
        }),
    );
    print_result(
        "verify p256 submit",
        iterations,
        time_loop(iterations, || {
            black_box(verify_submit(black_box(&p256_submit)));
        }),
    );
}

fn bench_e2e(iterations: usize, payload_len: usize) {
    let relay = Relay::new();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind relay listener");
    let addr = listener.local_addr().expect("relay addr");
    let serving_relay = relay.clone();
    thread::spawn(move || {
        let _ = serving_relay.serve_listener(listener);
    });

    let recipient = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let sender = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let recipient_identity = generated_p256_identity(&recipient);
    let payload = vec![0xA5; payload_len];
    let start = Instant::now();
    let mut completed = 0usize;
    let mut batches = 0usize;
    while completed < iterations {
        batches += 1;
        let batch_len = (iterations - completed).min(E2E_BATCH_DELIVERIES);
        let recipient_register = register_generated_p256(&recipient, batches as u64);

        let mut recipient_stream = connect_bench_stream(addr);
        let mut sender_stream = connect_bench_stream(addr);

        write_message(
            &mut recipient_stream,
            &RelayMessage::Register(recipient_register),
        )
        .expect("register recipient");
        expect_ack(&mut recipient_stream, 200);

        for offset in 0..batch_len {
            let i = completed + offset;
            let message_id = message_id_for(i as u64);
            let submit =
                submit_generated_p256(&sender, recipient_identity.clone(), message_id, &payload);
            write_message(&mut sender_stream, &RelayMessage::Submit(submit)).expect("send submit");

            let request = match read_message(&mut recipient_stream).expect("read delivery request")
            {
                RelayMessage::DeliveryRequest(request) => request,
                other => panic!("expected delivery request, got {other:?}"),
            };
            write_message(
                &mut recipient_stream,
                &RelayMessage::DeliveryReceipt(delivery_receipt(&recipient, &request, i as u64)),
            )
            .expect("send delivery receipt");
            expect_ack(&mut recipient_stream, 202);

            let report = read_delivery_report(&mut sender_stream);
            write_message(
                &mut sender_stream,
                &RelayMessage::DeliveryReportReceipt(report_receipt(&sender, &report, i as u64)),
            )
            .expect("send report receipt");
            expect_ack(&mut sender_stream, 200);
        }
        completed += batch_len;
    }
    let elapsed = start.elapsed();
    println!(
        "e2e_tcp_generated_p256 iterations={iterations} payload_len={payload_len} batches={batches} registered_nodes={} elapsed={elapsed:?}",
        relay.registered_len()
    );
    print_result("e2e generated p256 tcp delivery", iterations, elapsed);
}

fn bench_e2e_pipeline(iterations: usize, payload_len: usize, window: usize) {
    let relay = Relay::new();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind relay listener");
    let addr = listener.local_addr().expect("relay addr");
    let serving_relay = relay.clone();
    thread::spawn(move || {
        let _ = serving_relay.serve_listener(listener);
    });

    let elapsed = run_e2e_pipeline(addr, iterations, payload_len, window);
    println!(
        "e2e_tcp_generated_p256_pipeline iterations={iterations} payload_len={payload_len} window={window} registered_nodes={} elapsed={elapsed:?}",
        relay.registered_len()
    );
    print_result("e2e generated p256 tcp pipeline", iterations, elapsed);
}

fn bench_e2e_precomputed(iterations: usize, payload_len: usize, window: usize) {
    let relay = Relay::new();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind relay listener");
    let addr = listener.local_addr().expect("relay addr");
    let serving_relay = relay.clone();
    thread::spawn(move || {
        let _ = serving_relay.serve_listener(listener);
    });

    let elapsed = run_e2e_precomputed(addr, iterations, payload_len, window);
    println!(
        "e2e_tcp_generated_p256_precomputed iterations={iterations} payload_len={payload_len} window={window} registered_nodes={} elapsed={elapsed:?}",
        relay.registered_len()
    );
    print_result("e2e precomputed generated p256 tcp", iterations, elapsed);
}

fn bench_client_e2e_pipeline(
    addr: std::net::SocketAddr,
    iterations: usize,
    payload_len: usize,
    window: usize,
) {
    let elapsed = run_e2e_pipeline(addr, iterations, payload_len, window);
    println!(
        "client_e2e_tcp_generated_p256_pipeline addr={addr} iterations={iterations} payload_len={payload_len} window={window} elapsed={elapsed:?}"
    );
    print_result(
        "client e2e generated p256 tcp pipeline",
        iterations,
        elapsed,
    );
}

fn bench_client_udp_e2e(addr: SocketAddr, iterations: usize, payload_len: usize) {
    let recipient = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let sender = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let recipient_identity = generated_p256_identity(&recipient);
    let payload = vec![0xA5; payload_len];
    let recipient_socket = bind_bench_udp();
    let sender_socket = bind_bench_udp();

    write_udp(
        &recipient_socket,
        addr,
        &RelayMessage::Register(register_generated_p256(&recipient, 1)),
    );
    expect_udp_ack(&recipient_socket, 200);

    let start = Instant::now();
    for i in 0..iterations {
        let submit = submit_generated_p256(
            &sender,
            recipient_identity.clone(),
            message_id_for(i as u64),
            &payload,
        );
        write_udp(&sender_socket, addr, &RelayMessage::Submit(submit));

        let request = match read_udp(&recipient_socket) {
            RelayMessage::DeliveryRequest(request) => request,
            other => panic!("expected udp delivery request, got {other:?}"),
        };
        write_udp(
            &recipient_socket,
            addr,
            &RelayMessage::DeliveryReceipt(delivery_receipt(&recipient, &request, i as u64)),
        );
        expect_udp_ack(&recipient_socket, 202);

        let report = read_udp_delivery_report(&sender_socket);
        write_udp(
            &sender_socket,
            addr,
            &RelayMessage::DeliveryReportReceipt(report_receipt(&sender, &report, i as u64)),
        );
        expect_udp_ack(&sender_socket, 200);
    }
    let elapsed = start.elapsed();
    println!(
        "client_udp_e2e_generated_p256 addr={addr} iterations={iterations} payload_len={payload_len} elapsed={elapsed:?}"
    );
    print_result("client udp e2e generated p256", iterations, elapsed);
}

fn bench_client_udp_pipeline(
    addr: SocketAddr,
    iterations: usize,
    payload_len: usize,
    window: usize,
) {
    let recipient = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let sender = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let recipient_identity = generated_p256_identity(&recipient);
    let payload = vec![0xA5; payload_len];
    let recipient_socket = bind_bench_udp();
    let sender_socket = bind_bench_udp();

    write_udp(
        &recipient_socket,
        addr,
        &RelayMessage::Register(register_generated_p256(&recipient, 1)),
    );
    expect_udp_ack(&recipient_socket, 200);

    let start = Instant::now();
    let mut completed = 0usize;
    while completed < iterations {
        let batch_len = (iterations - completed).min(window);
        let mut requests: Vec<edgerun_wire::RelayDeliveryRequest> = Vec::with_capacity(batch_len);
        let mut reports: Vec<RelayDeliveryReport> = Vec::with_capacity(batch_len);
        let mut submit_acks = 0usize;

        for offset in 0..batch_len {
            let i = completed + offset;
            let submit = submit_generated_p256(
                &sender,
                recipient_identity.clone(),
                message_id_for(i as u64),
                &payload,
            );
            write_udp(&sender_socket, addr, &RelayMessage::Submit(submit));
        }

        while requests.len() < batch_len {
            match read_udp(&recipient_socket) {
                RelayMessage::DeliveryRequest(request) => requests.push(request),
                other => panic!("expected udp pipelined delivery request, got {other:?}"),
            }
        }

        for (offset, request) in requests.iter().enumerate() {
            write_udp(
                &recipient_socket,
                addr,
                &RelayMessage::DeliveryReceipt(delivery_receipt(
                    &recipient,
                    request,
                    (completed + offset) as u64,
                )),
            );
        }
        for _ in 0..batch_len {
            expect_udp_ack(&recipient_socket, 202);
        }

        while reports.len() < batch_len {
            match read_udp(&sender_socket) {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                RelayMessage::DeliveryReport(report) => reports.push(report),
                other => panic!("expected udp submit ack or delivery report, got {other:?}"),
            }
        }
        while submit_acks < batch_len {
            match read_udp(&sender_socket) {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                other => panic!("expected remaining udp submit ack, got {other:?}"),
            }
        }

        for (offset, report) in reports.iter().enumerate() {
            write_udp(
                &sender_socket,
                addr,
                &RelayMessage::DeliveryReportReceipt(report_receipt(
                    &sender,
                    report,
                    (completed + offset) as u64,
                )),
            );
        }
        for _ in 0..batch_len {
            expect_udp_ack(&sender_socket, 200);
        }

        completed += batch_len;
    }
    let elapsed = start.elapsed();
    println!(
        "client_udp_pipeline_generated_p256 addr={addr} iterations={iterations} payload_len={payload_len} window={window} elapsed={elapsed:?}"
    );
    print_result("client udp pipeline generated p256", iterations, elapsed);
}

fn bench_client_custom_quic(
    addr: SocketAddr,
    iterations: usize,
    payload_len: usize,
    window: usize,
) {
    let recipient = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let sender = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let recipient_identity = generated_p256_identity(&recipient);
    let payload = vec![0xA5; payload_len];
    let recipient_socket = bind_bench_udp();
    let sender_socket = bind_bench_udp();
    let recipient_cid = 0xED6E_0000_0000_0001;
    let sender_cid = 0xED6E_0000_0000_0002;
    let mut recipient_packet_number = 0u64;
    let mut sender_packet_number = 0u64;
    let mut recipient_outbox = Vec::new();
    let mut sender_outbox = Vec::new();
    let mut recipient_inbox = Vec::new();
    let mut sender_inbox = Vec::new();

    write_custom_quic(
        &recipient_socket,
        addr,
        recipient_cid,
        &mut recipient_packet_number,
        &mut recipient_outbox,
        &RelayMessage::Register(register_generated_p256(&recipient, 1)),
    );
    expect_custom_quic_ack(
        &recipient_socket,
        addr,
        recipient_cid,
        &mut recipient_outbox,
        &mut recipient_inbox,
        200,
    );

    let start = Instant::now();
    let mut completed = 0usize;
    while completed < iterations {
        let batch_len = (iterations - completed).min(window);
        let mut requests: Vec<edgerun_wire::RelayDeliveryRequest> = Vec::with_capacity(batch_len);
        let mut reports: Vec<RelayDeliveryReport> = Vec::with_capacity(batch_len);
        let mut submit_acks = 0usize;

        for offset in 0..batch_len {
            let i = completed + offset;
            let submit = submit_generated_p256(
                &sender,
                recipient_identity.clone(),
                message_id_for(i as u64),
                &payload,
            );
            write_custom_quic(
                &sender_socket,
                addr,
                sender_cid,
                &mut sender_packet_number,
                &mut sender_outbox,
                &RelayMessage::Submit(submit),
            );
        }

        while requests.len() < batch_len {
            match read_custom_quic(
                &recipient_socket,
                addr,
                recipient_cid,
                &mut recipient_outbox,
                &mut recipient_inbox,
            ) {
                RelayMessage::DeliveryRequest(request) => {
                    if !requests
                        .iter()
                        .any(|seen: &edgerun_wire::RelayDeliveryRequest| {
                            seen.submit.message_id == request.submit.message_id
                        })
                    {
                        requests.push(request);
                    }
                }
                RelayMessage::Ack(RelayAck { ok: true, .. }) => {}
                other => panic!("expected custom quic delivery request, got {other:?}"),
            }
        }

        for (offset, request) in requests.iter().enumerate() {
            write_custom_quic(
                &recipient_socket,
                addr,
                recipient_cid,
                &mut recipient_packet_number,
                &mut recipient_outbox,
                &RelayMessage::DeliveryReceipt(delivery_receipt(
                    &recipient,
                    request,
                    (completed + offset) as u64,
                )),
            );
        }
        for _ in 0..batch_len {
            expect_custom_quic_ack(
                &recipient_socket,
                addr,
                recipient_cid,
                &mut recipient_outbox,
                &mut recipient_inbox,
                202,
            );
        }

        while reports.len() < batch_len {
            match read_custom_quic(
                &sender_socket,
                addr,
                sender_cid,
                &mut sender_outbox,
                &mut sender_inbox,
            ) {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                RelayMessage::Ack(RelayAck { ok: true, .. }) => {}
                RelayMessage::DeliveryReport(report) => {
                    if !reports.iter().any(|seen: &RelayDeliveryReport| {
                        seen.submit.message_id == report.submit.message_id
                    }) {
                        reports.push(report);
                    }
                }
                other => {
                    panic!("expected custom quic submit ack or delivery report, got {other:?}")
                }
            }
        }
        while submit_acks < batch_len {
            match read_custom_quic(
                &sender_socket,
                addr,
                sender_cid,
                &mut sender_outbox,
                &mut sender_inbox,
            ) {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                RelayMessage::Ack(RelayAck { ok: true, .. }) => {}
                RelayMessage::DeliveryReport(_) => {}
                other => panic!("expected remaining custom quic submit ack, got {other:?}"),
            }
        }

        for (offset, report) in reports.iter().enumerate() {
            write_custom_quic(
                &sender_socket,
                addr,
                sender_cid,
                &mut sender_packet_number,
                &mut sender_outbox,
                &RelayMessage::DeliveryReportReceipt(report_receipt(
                    &sender,
                    report,
                    (completed + offset) as u64,
                )),
            );
        }
        for _ in 0..batch_len {
            expect_custom_quic_ack(
                &sender_socket,
                addr,
                sender_cid,
                &mut sender_outbox,
                &mut sender_inbox,
                200,
            );
        }

        completed += batch_len;
    }
    let elapsed = start.elapsed();
    println!(
        "client_custom_quic_generated_p256 addr={addr} iterations={iterations} payload_len={payload_len} window={window} elapsed={elapsed:?}"
    );
    print_result("client custom quic generated p256", iterations, elapsed);
}

fn run_e2e_pipeline(
    addr: std::net::SocketAddr,
    iterations: usize,
    payload_len: usize,
    window: usize,
) -> Duration {
    let recipient = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let sender = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let recipient_identity = generated_p256_identity(&recipient);
    let payload = vec![0xA5; payload_len];

    let mut recipient_stream = connect_bench_stream(addr);
    let mut sender_stream = connect_bench_stream(addr);
    write_message(
        &mut recipient_stream,
        &RelayMessage::Register(register_generated_p256(&recipient, 1)),
    )
    .expect("register recipient");
    expect_ack(&mut recipient_stream, 200);

    let start = Instant::now();
    let mut completed = 0usize;
    while completed < iterations {
        let batch_len = (iterations - completed).min(window);
        let mut requests = Vec::with_capacity(batch_len);
        let mut reports = Vec::with_capacity(batch_len);
        let mut submit_acks = 0usize;

        for offset in 0..batch_len {
            let i = completed + offset;
            let submit = submit_generated_p256(
                &sender,
                recipient_identity.clone(),
                message_id_for(i as u64),
                &payload,
            );
            write_message(&mut sender_stream, &RelayMessage::Submit(submit)).expect("send submit");
        }

        while requests.len() < batch_len {
            match read_message(&mut recipient_stream).expect("read pipelined delivery request") {
                RelayMessage::DeliveryRequest(request) => requests.push(request),
                other => panic!("expected pipelined delivery request, got {other:?}"),
            }
        }

        for (offset, request) in requests.iter().enumerate() {
            write_message(
                &mut recipient_stream,
                &RelayMessage::DeliveryReceipt(delivery_receipt(
                    &recipient,
                    request,
                    (completed + offset) as u64,
                )),
            )
            .expect("send delivery receipt");
        }
        for _ in 0..batch_len {
            expect_ack(&mut recipient_stream, 202);
        }

        while reports.len() < batch_len {
            match read_message(&mut sender_stream).expect("read pipelined sender message") {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                RelayMessage::DeliveryReport(report) => reports.push(report),
                other => panic!("expected submit ack or delivery report, got {other:?}"),
            }
        }
        while submit_acks < batch_len {
            match read_message(&mut sender_stream).expect("read remaining submit ack") {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                other => panic!("expected remaining submit ack, got {other:?}"),
            }
        }

        for (offset, report) in reports.iter().enumerate() {
            write_message(
                &mut sender_stream,
                &RelayMessage::DeliveryReportReceipt(report_receipt(
                    &sender,
                    report,
                    (completed + offset) as u64,
                )),
            )
            .expect("send report receipt");
        }
        for _ in 0..batch_len {
            expect_ack(&mut sender_stream, 200);
        }

        completed += batch_len;
    }

    start.elapsed()
}

fn run_e2e_precomputed(
    addr: std::net::SocketAddr,
    iterations: usize,
    payload_len: usize,
    window: usize,
) -> Duration {
    let recipient = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let sender = edgerun_protocols::keygen::generate_ephemeral_node_identity();
    let recipient_identity = generated_p256_identity(&recipient);
    let payload = vec![0xA5; payload_len];

    let mut recipient_stream = connect_bench_stream(addr);
    let mut sender_stream = connect_bench_stream(addr);
    write_message(
        &mut recipient_stream,
        &RelayMessage::Register(register_generated_p256(&recipient, 1)),
    )
    .expect("register recipient");
    expect_ack(&mut recipient_stream, 200);

    let mut submit_frames = Vec::with_capacity(iterations);
    let mut receipt_frames = Vec::with_capacity(iterations);
    let mut report_receipt_frames = Vec::with_capacity(iterations);
    for i in 0..iterations {
        let submit = submit_generated_p256(
            &sender,
            recipient_identity.clone(),
            message_id_for(i as u64),
            &payload,
        );
        let request = RelayDeliveryRequest {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            relay_id: b"edgerun-relay".to_vec(),
            submit: submit.clone(),
            received_unix_ms: 0,
        };
        let receipt = delivery_receipt(&recipient, &request, i as u64);
        let report = RelayDeliveryReport {
            abi_version: RELAY_WIRE_ABI_VERSION,
            flags: 1,
            relay_id: b"edgerun-relay".to_vec(),
            submit: submit.clone(),
            recipient_receipt: receipt.clone(),
            reported_unix_ms: 0,
        };
        submit_frames.push(encode_stream_message(&RelayMessage::Submit(submit)));
        receipt_frames.push(encode_stream_message(&RelayMessage::DeliveryReceipt(
            receipt,
        )));
        report_receipt_frames.push(encode_stream_message(&RelayMessage::DeliveryReportReceipt(
            report_receipt(&sender, &report, i as u64),
        )));
    }

    let start = Instant::now();
    let mut completed = 0usize;
    while completed < iterations {
        let batch_len = (iterations - completed).min(window);
        let mut reports = 0usize;
        let mut submit_acks = 0usize;

        for frame in &submit_frames[completed..completed + batch_len] {
            sender_stream.write_all(frame).expect("send submit frame");
        }

        for _ in 0..batch_len {
            match read_message(&mut recipient_stream).expect("read precomputed delivery request") {
                RelayMessage::DeliveryRequest(_) => {}
                other => panic!("expected precomputed delivery request, got {other:?}"),
            }
        }

        for frame in &receipt_frames[completed..completed + batch_len] {
            recipient_stream
                .write_all(frame)
                .expect("send receipt frame");
        }
        for _ in 0..batch_len {
            expect_ack(&mut recipient_stream, 202);
        }

        while reports < batch_len {
            match read_message(&mut sender_stream).expect("read precomputed sender message") {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                RelayMessage::DeliveryReport(_) => reports += 1,
                other => panic!("expected precomputed submit ack or report, got {other:?}"),
            }
        }
        while submit_acks < batch_len {
            match read_message(&mut sender_stream).expect("read precomputed remaining submit ack") {
                RelayMessage::Ack(RelayAck {
                    ok: true,
                    code: 202,
                    ..
                }) => submit_acks += 1,
                other => panic!("expected precomputed remaining submit ack, got {other:?}"),
            }
        }

        for frame in &report_receipt_frames[completed..completed + batch_len] {
            sender_stream
                .write_all(frame)
                .expect("send report receipt frame");
        }
        for _ in 0..batch_len {
            expect_ack(&mut sender_stream, 200);
        }

        completed += batch_len;
    }

    start.elapsed()
}

fn time_loop(iterations: usize, mut f: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed()
}

fn print_result(label: &str, iterations: usize, elapsed: Duration) {
    let seconds = elapsed.as_secs_f64();
    let ops_per_second = iterations as f64 / seconds;
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("{label}: {ops_per_second:.0} ops/s, {ns_per_op:.1} ns/op, {elapsed:?}");
}

fn connect_bench_stream(addr: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(addr).expect("connect relay");
    stream.set_nodelay(true).expect("disable nagle");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("read timeout");
    stream
}

fn bind_bench_udp() -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").expect("bind udp client");
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("udp read timeout");
    socket
}

fn encode_stream_message(message: &RelayMessage) -> Vec<u8> {
    let bytes = encode_packet(message).expect("encode stream relay packet");
    let mut frame = Vec::with_capacity(4 + bytes.len());
    frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    frame.extend_from_slice(&bytes);
    frame
}

fn ed25519_identity(seed: u8) -> (Ed25519SigningKey, RelayIdentity) {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let identity = RelayIdentity {
        algorithm: SIGNATURE_ALGORITHM_ED25519,
        public_key: key.verifying_key().as_bytes().to_vec(),
    };
    (key, identity)
}

fn generated_p256_identity(node: &EphemeralNodeIdentity) -> RelayIdentity {
    RelayIdentity {
        algorithm: SIGNATURE_ALGORITHM_ECDSA_P256_SHA256,
        public_key: node.node_id.to_vec(),
    }
}

fn sign_ed25519(
    key: &Ed25519SigningKey,
    identity: &RelayIdentity,
    preimage: &[u8],
) -> RelaySignature {
    RelaySignature {
        algorithm: identity.algorithm,
        public_key: identity.public_key.clone(),
        signature: key.sign(preimage).to_bytes().to_vec(),
    }
}

fn sign_generated_p256(
    node: &EphemeralNodeIdentity,
    identity: &RelayIdentity,
    preimage: &[u8],
) -> RelaySignature {
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner as _;
    let digest = edgerun_crypto::sha256(preimage);
    let signature: edgerun_crypto::p256::ecdsa::Signature = node
        .signer
        .signing_key()
        .sign_prehash(&digest)
        .expect("p256 sign");
    RelaySignature {
        algorithm: identity.algorithm,
        public_key: identity.public_key.clone(),
        signature: signature.to_bytes().to_vec(),
    }
}

fn register_generated_p256(node: &EphemeralNodeIdentity, sequence: u64) -> RelayRegister {
    let identity = generated_p256_identity(node);
    let mut register = RelayRegister {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        node: identity,
        sequence,
        log_head: [0xA5; 32],
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    register.signature = sign_generated_p256(
        node,
        &register.node,
        &edgerun_relay::register_preimage(&register),
    );
    register
}

fn submit_ed25519(
    sender_seed: u8,
    to: RelayIdentity,
    message_id: [u8; 32],
    payload: &[u8],
) -> RelaySubmit {
    let (key, from) = ed25519_identity(sender_seed);
    let mut submit = RelaySubmit {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        message_id,
        from,
        to,
        sequence: 7,
        payload_sha256: sha256_array(payload),
        payload: payload.to_vec(),
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    submit.signature = sign_ed25519(&key, &submit.from, &submit_preimage(&submit));
    submit
}

fn submit_generated_p256(
    sender: &EphemeralNodeIdentity,
    to: RelayIdentity,
    message_id: [u8; 32],
    payload: &[u8],
) -> RelaySubmit {
    let from = generated_p256_identity(sender);
    let mut submit = RelaySubmit {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        message_id,
        from,
        to,
        sequence: 7,
        payload_sha256: sha256_array(payload),
        payload: payload.to_vec(),
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    submit.signature = sign_generated_p256(sender, &submit.from, &submit_preimage(&submit));
    submit
}

fn submit_p256(
    sender_seed: u8,
    to: RelayIdentity,
    message_id: [u8; 32],
    payload: &[u8],
) -> RelaySubmit {
    let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes((&[sender_seed; 32]).into())
        .expect("p256 key");
    let from = RelayIdentity {
        algorithm: SIGNATURE_ALGORITHM_ECDSA_P256_SHA256,
        public_key: edgerun_protocols::keygen::node_id_from_signing_key(&key).to_vec(),
    };
    let mut submit = RelaySubmit {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        message_id,
        from,
        to,
        sequence: 7,
        payload_sha256: sha256_array(payload),
        payload: payload.to_vec(),
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    let digest = edgerun_crypto::sha256(&submit_preimage(&submit));
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner as _;
    let signature: edgerun_crypto::p256::ecdsa::Signature =
        key.sign_prehash(&digest).expect("p256 sign");
    submit.signature = RelaySignature {
        algorithm: submit.from.algorithm,
        public_key: submit.from.public_key.clone(),
        signature: signature.to_bytes().to_vec(),
    };
    submit
}

fn delivery_receipt(
    recipient: &EphemeralNodeIdentity,
    request: &edgerun_wire::RelayDeliveryRequest,
    sequence: u64,
) -> RelayDeliveryReceipt {
    let mut receipt = RelayDeliveryReceipt {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        message_id: request.submit.message_id,
        recipient: generated_p256_identity(recipient),
        status: RELAY_DELIVERY_STATUS_ACCEPTED,
        recipient_sequence: sequence,
        recipient_log_head: [0xB1; 32],
        request_sha256: request_hash(request),
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    receipt.signature = sign_generated_p256(
        recipient,
        &receipt.recipient,
        &delivery_receipt_preimage(&receipt),
    );
    receipt
}

fn report_receipt(
    sender: &EphemeralNodeIdentity,
    report: &RelayDeliveryReport,
    sequence: u64,
) -> RelayDeliveryReportReceipt {
    let mut receipt = RelayDeliveryReportReceipt {
        abi_version: RELAY_WIRE_ABI_VERSION,
        flags: 1,
        message_id: report.submit.message_id,
        sender: generated_p256_identity(sender),
        status: RELAY_REPORT_STATUS_ACCEPTED,
        sender_sequence: sequence,
        sender_log_head: [0xC1; 32],
        report_sha256: report_hash(report),
        signature: RelaySignature {
            algorithm: 0,
            public_key: Vec::new(),
            signature: Vec::new(),
        },
    };
    receipt.signature =
        sign_generated_p256(sender, &receipt.sender, &report_receipt_preimage(&receipt));
    receipt
}

fn read_delivery_report(stream: &mut TcpStream) -> RelayDeliveryReport {
    match read_message(stream).expect("read delivery report") {
        RelayMessage::Ack(RelayAck { code: 202, .. }) => {
            match read_message(stream).expect("read report after ack") {
                RelayMessage::DeliveryReport(report) => report,
                other => panic!("expected delivery report after ack, got {other:?}"),
            }
        }
        RelayMessage::DeliveryReport(report) => report,
        other => panic!("expected delivery report, got {other:?}"),
    }
}

fn write_udp(socket: &UdpSocket, relay_addr: SocketAddr, message: &RelayMessage) {
    let bytes = encode_packet(message).expect("encode udp relay packet");
    socket
        .send_to(&bytes, relay_addr)
        .expect("send udp relay packet");
}

fn read_udp(socket: &UdpSocket) -> RelayMessage {
    let mut buf = vec![0u8; edgerun_relay::MAX_FRAME_LEN];
    let (len, _) = socket.recv_from(&mut buf).expect("read udp relay packet");
    edgerun_relay::decode_packet(&buf[..len]).expect("decode udp relay packet")
}

const CUSTOM_QUIC_MAGIC: &[u8; 4] = b"ERQ0";
const CUSTOM_QUIC_DATA: u8 = 1;
const CUSTOM_QUIC_ACK: u8 = 2;
const CUSTOM_QUIC_HEADER_LEN: usize = 4 + 1 + 8 + 8 + 2;
const CUSTOM_QUIC_BATCH_MAGIC: &[u8; 4] = b"ERQB";
const CUSTOM_QUIC_MAX_RELAY_PAYLOAD: usize = 1100;

struct CustomQuicSent {
    packet_number: u64,
}

fn write_custom_quic(
    socket: &UdpSocket,
    relay_addr: SocketAddr,
    connection_id: u64,
    packet_number: &mut u64,
    outbox: &mut Vec<CustomQuicSent>,
    message: &RelayMessage,
) {
    let relay_payload = encode_packet(message).expect("encode custom quic relay packet");
    let bytes = custom_quic_encode_data(connection_id, *packet_number, &relay_payload);
    outbox.push(CustomQuicSent {
        packet_number: *packet_number,
    });
    *packet_number = packet_number.wrapping_add(1);
    socket
        .send_to(&bytes, relay_addr)
        .expect("send custom quic relay packet");
}

fn write_custom_quic_batch(
    socket: &UdpSocket,
    relay_addr: SocketAddr,
    connection_id: u64,
    packet_number: &mut u64,
    outbox: &mut Vec<CustomQuicSent>,
    messages: &[RelayMessage],
) {
    if messages.len() == 1 {
        write_custom_quic(
            socket,
            relay_addr,
            connection_id,
            packet_number,
            outbox,
            &messages[0],
        );
        return;
    }
    let mut chunk = Vec::new();
    let mut chunk_len = 6usize;
    for message in messages {
        let encoded = encode_packet(message).expect("encode custom quic relay batch item");
        let item_len = 4 + encoded.len();
        if !chunk.is_empty() && chunk_len + item_len > CUSTOM_QUIC_MAX_RELAY_PAYLOAD {
            write_custom_quic_encoded_batch(
                socket,
                relay_addr,
                connection_id,
                packet_number,
                outbox,
                &chunk,
            );
            chunk.clear();
            chunk_len = 6;
        }
        chunk_len += item_len;
        chunk.push(encoded);
    }
    if !chunk.is_empty() {
        write_custom_quic_encoded_batch(
            socket,
            relay_addr,
            connection_id,
            packet_number,
            outbox,
            &chunk,
        );
    }
}

fn write_custom_quic_encoded_batch(
    socket: &UdpSocket,
    relay_addr: SocketAddr,
    connection_id: u64,
    packet_number: &mut u64,
    outbox: &mut Vec<CustomQuicSent>,
    encoded_messages: &[Vec<u8>],
) {
    let relay_payload = custom_quic_encode_relay_batch(encoded_messages);
    let bytes = custom_quic_encode_data(connection_id, *packet_number, &relay_payload);
    outbox.push(CustomQuicSent {
        packet_number: *packet_number,
    });
    *packet_number = packet_number.wrapping_add(1);
    socket
        .send_to(&bytes, relay_addr)
        .expect("send custom quic relay batch");
}

fn read_custom_quic(
    socket: &UdpSocket,
    relay_addr: SocketAddr,
    connection_id: u64,
    outbox: &mut Vec<CustomQuicSent>,
    inbox: &mut Vec<RelayMessage>,
) -> RelayMessage {
    if !inbox.is_empty() {
        return inbox.remove(0);
    }
    let mut buf = vec![0u8; edgerun_relay::MAX_FRAME_LEN];
    loop {
        let (len, _) = socket.recv_from(&mut buf).expect("read custom quic packet");
        let packet = &buf[..len];
        if let Some((cid, packet_number)) = custom_quic_decode_ack(packet) {
            if cid == connection_id {
                outbox.retain(|packet| packet.packet_number != packet_number);
            }
            continue;
        }
        if let Some((cid, packet_number, payload)) = custom_quic_decode_data(packet) {
            if cid == connection_id {
                let ack = custom_quic_encode_ack(connection_id, packet_number);
                socket
                    .send_to(&ack, relay_addr)
                    .expect("send custom quic transport ack");
                let mut messages =
                    custom_quic_decode_relay_messages(payload).expect("decode custom quic relay");
                if messages.is_empty() {
                    continue;
                }
                inbox.extend(messages.drain(1..));
                return messages.remove(0);
            }
        }
    }
}

fn custom_quic_decode_data(payload: &[u8]) -> Option<(u64, u64, &[u8])> {
    if payload.len() < CUSTOM_QUIC_HEADER_LEN
        || &payload[..4] != CUSTOM_QUIC_MAGIC
        || payload[4] != CUSTOM_QUIC_DATA
    {
        return None;
    }
    let connection_id = u64::from_be_bytes(payload[5..13].try_into().ok()?);
    let packet_number = u64::from_be_bytes(payload[13..21].try_into().ok()?);
    let len = u16::from_be_bytes(payload[21..23].try_into().ok()?) as usize;
    let end = CUSTOM_QUIC_HEADER_LEN.checked_add(len)?;
    if end > payload.len() {
        return None;
    }
    Some((
        connection_id,
        packet_number,
        &payload[CUSTOM_QUIC_HEADER_LEN..end],
    ))
}

fn custom_quic_decode_ack(payload: &[u8]) -> Option<(u64, u64)> {
    if payload.len() < CUSTOM_QUIC_HEADER_LEN
        || &payload[..4] != CUSTOM_QUIC_MAGIC
        || payload[4] != CUSTOM_QUIC_ACK
    {
        return None;
    }
    let connection_id = u64::from_be_bytes(payload[5..13].try_into().ok()?);
    let packet_number = u64::from_be_bytes(payload[13..21].try_into().ok()?);
    Some((connection_id, packet_number))
}

fn custom_quic_decode_relay_messages(payload: &[u8]) -> Option<Vec<RelayMessage>> {
    if payload.len() < 6 || &payload[..4] != CUSTOM_QUIC_BATCH_MAGIC {
        return edgerun_relay::decode_packet(payload)
            .ok()
            .map(|message| vec![message]);
    }
    let count = u16::from_be_bytes(payload[4..6].try_into().ok()?) as usize;
    let mut offset = 6usize;
    let mut messages = Vec::with_capacity(count);
    for _ in 0..count {
        if payload.len().saturating_sub(offset) < 4 {
            return None;
        }
        let len = u32::from_be_bytes(payload[offset..offset + 4].try_into().ok()?) as usize;
        offset = offset.checked_add(4)?;
        let end = offset.checked_add(len)?;
        if end > payload.len() {
            return None;
        }
        messages.push(edgerun_relay::decode_packet(&payload[offset..end]).ok()?);
        offset = end;
    }
    Some(messages)
}

fn custom_quic_encode_relay_batch(encoded: &[Vec<u8>]) -> Vec<u8> {
    let len = encoded.iter().map(|item| 4 + item.len()).sum::<usize>();
    let mut out = Vec::with_capacity(6 + len);
    out.extend_from_slice(CUSTOM_QUIC_BATCH_MAGIC);
    out.extend_from_slice(&(encoded.len().min(u16::MAX as usize) as u16).to_be_bytes());
    for item in encoded {
        out.extend_from_slice(&(item.len() as u32).to_be_bytes());
        out.extend_from_slice(&item);
    }
    out
}

fn custom_quic_encode_data(
    connection_id: u64,
    packet_number: u64,
    relay_payload: &[u8],
) -> Vec<u8> {
    let len = relay_payload.len().min(u16::MAX as usize);
    let mut out = Vec::with_capacity(CUSTOM_QUIC_HEADER_LEN + len);
    out.extend_from_slice(CUSTOM_QUIC_MAGIC);
    out.push(CUSTOM_QUIC_DATA);
    out.extend_from_slice(&connection_id.to_be_bytes());
    out.extend_from_slice(&packet_number.to_be_bytes());
    out.extend_from_slice(&(len as u16).to_be_bytes());
    out.extend_from_slice(&relay_payload[..len]);
    out
}

fn custom_quic_encode_ack(connection_id: u64, packet_number: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(CUSTOM_QUIC_HEADER_LEN);
    out.extend_from_slice(CUSTOM_QUIC_MAGIC);
    out.push(CUSTOM_QUIC_ACK);
    out.extend_from_slice(&connection_id.to_be_bytes());
    out.extend_from_slice(&packet_number.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out
}

fn read_udp_delivery_report(socket: &UdpSocket) -> RelayDeliveryReport {
    match read_udp(socket) {
        RelayMessage::Ack(RelayAck { code: 202, .. }) => match read_udp(socket) {
            RelayMessage::DeliveryReport(report) => report,
            other => panic!("expected udp delivery report after ack, got {other:?}"),
        },
        RelayMessage::DeliveryReport(report) => report,
        other => panic!("expected udp delivery report, got {other:?}"),
    }
}

fn expect_ack(stream: &mut TcpStream, code: u16) {
    match read_message(stream).expect("read ack") {
        RelayMessage::Ack(RelayAck {
            ok: true,
            code: actual,
            ..
        }) if actual == code => {}
        other => panic!("expected ack {code}, got {other:?}"),
    }
}

fn expect_udp_ack(socket: &UdpSocket, code: u16) {
    match read_udp(socket) {
        RelayMessage::Ack(RelayAck {
            ok: true,
            code: actual,
            ..
        }) if actual == code => {}
        other => panic!("expected udp ack {code}, got {other:?}"),
    }
}

fn expect_custom_quic_ack(
    socket: &UdpSocket,
    relay_addr: SocketAddr,
    connection_id: u64,
    outbox: &mut Vec<CustomQuicSent>,
    inbox: &mut Vec<RelayMessage>,
    code: u16,
) {
    loop {
        match read_custom_quic(socket, relay_addr, connection_id, outbox, inbox) {
            RelayMessage::Ack(RelayAck {
                ok: true,
                code: actual,
                ..
            }) if actual == code => return,
            RelayMessage::Ack(RelayAck { ok: true, .. }) => {}
            RelayMessage::DeliveryRequest(_) | RelayMessage::DeliveryReport(_) => {}
            other => panic!("expected custom quic ack {code}, got {other:?}"),
        }
    }
}

fn message_id_for(value: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0..8].copy_from_slice(&value.to_be_bytes());
    id[8..16].copy_from_slice(&(!value).to_be_bytes());
    id[16..24].copy_from_slice(&value.rotate_left(17).to_be_bytes());
    id[24..32].copy_from_slice(&0xED6E_0000_0000_0001u64.wrapping_add(value).to_be_bytes());
    id
}
