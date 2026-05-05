use std::hint::black_box;
use std::time::Instant;

use edgerun_sign_verify_e2e::{sign_event_only, sign_verify_event_roundtrip, verify_event_only};

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

    let report = sign_verify_event_roundtrip().expect("roundtrip should pass");
    println!(
        "roundtrip: signature_len={} record_hash_len={} public_key_len={} stream_id_len={}",
        report.signature_len, report.record_hash_len, report.public_key_len, report.stream_id_len
    );

    bench_once("sign_event", iterations, |n| {
        let signatures = sign_event_only(n).expect("sign benchmark should pass");
        black_box(signatures);
    });

    bench_once("sign_plus_verify_event", iterations, |n| {
        let ok = verify_event_only(n).expect("verify benchmark should pass");
        black_box(ok);
    });
}
