use std::hint::black_box;
use std::time::Instant;

use edgerun_sign_verify_e2e::{
    agent_node_only, agent_node_roundtrip, bootstrap_only, bootstrap_roundtrip, hash_event_only,
    sign_event_only, sign_verify_event_roundtrip, signed_events, stream_only, stream_roundtrip,
    verify_event_only, verify_prebuilt_events, wire_event_only,
};

fn bench_once(name: &str, iterations: usize, mut f: impl FnMut(usize)) {
    let start = Instant::now();
    f(iterations);
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let ops_per_sec = if secs > 0.0 {
        iterations as f64 / secs
    } else {
        f64::INFINITY
    };
    let micros_per_op = if iterations > 0 {
        elapsed.as_micros() as f64 / iterations as f64
    } else {
        0.0
    };

    println!(
        "{name}: iterations={iterations} total_ms={:.3} ops_per_sec={:.2} micros_per_op={:.3}",
        elapsed.as_secs_f64() * 1000.0,
        ops_per_sec,
        micros_per_op
    );
}

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1_000);

    let bootstrap = bootstrap_roundtrip().expect("bootstrap roundtrip should pass");
    println!(
        "bootstrap: node_id_len={} stored_key_len={} genesis_signature_len={} genesis_stream_id_len={} genesis_seq={} store_len={}",
        bootstrap.node_id_len,
        bootstrap.stored_key_len,
        bootstrap.genesis_signature_len,
        bootstrap.genesis_stream_id_len,
        bootstrap.genesis_seq,
        bootstrap.store_len,
    );

    let stream = stream_roundtrip().expect("stream roundtrip should pass");
    println!(
        "stream: event_count={} genesis_seq={} next_seq={} next_has_prev_hash={} next_signature_len={}",
        stream.event_count,
        stream.genesis_seq,
        stream.next_seq,
        stream.next_has_prev_hash,
        stream.next_signature_len,
    );

    let agent = agent_node_roundtrip().expect("agent node roundtrip should pass");
    println!(
        "agent_node: node_id_len={} store_len={} event_count={} genesis_seq={} action_started_seq={} action_completed_seq={} action_completed_has_prev_hash={} all_events_signed={}",
        agent.node_id_len,
        agent.store_len,
        agent.event_count,
        agent.genesis_seq,
        agent.action_started_seq,
        agent.action_completed_seq,
        agent.action_completed_has_prev_hash,
        agent.all_events_signed,
    );

    let report = sign_verify_event_roundtrip().expect("roundtrip should pass");
    println!(
        "roundtrip: signature_len={} record_hash_len={} public_key_len={} stream_id_len={}",
        report.signature_len, report.record_hash_len, report.public_key_len, report.stream_id_len
    );

    bench_once("bootstrap", iterations, |n| {
        let ok = bootstrap_only(n).expect("bootstrap benchmark should pass");
        black_box(ok);
    });

    bench_once("stream", iterations, |n| {
        let ok = stream_only(n).expect("stream benchmark should pass");
        black_box(ok);
    });

    bench_once("agent_node", iterations, |n| {
        let ok = agent_node_only(n).expect("agent node benchmark should pass");
        black_box(ok);
    });

    bench_once("sign_event", iterations, |n| {
        let signatures = sign_event_only(n).expect("sign benchmark should pass");
        black_box(signatures);
    });

    bench_once("sign_plus_verify_event", iterations, |n| {
        let ok = verify_event_only(n).expect("verify benchmark should pass");
        black_box(ok);
    });

    let (events, public_key) =
        signed_events(iterations).expect("prebuild signed events should pass");

    bench_once("verify_event_only", iterations, |n| {
        let ok = verify_prebuilt_events(&events[..n], &public_key)
            .expect("verify-only benchmark should pass");
        black_box(ok);
    });

    bench_once("wire_event", iterations, |n| {
        let bytes = wire_event_only(n);
        black_box(bytes);
    });

    bench_once("hash_event", iterations, |n| {
        let bytes = hash_event_only(n);
        black_box(bytes);
    });
}
