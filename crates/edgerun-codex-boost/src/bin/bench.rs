use edgerun_codex_boost::access_record;
use edgerun_codex_boost::archive_record;
use edgerun_codex_boost::decode_record;
use edgerun_codex_boost::CodexBoostRecord;
use edgerun_json::parse_json;
use edgerun_json::parse_json_tape;
use edgerun_json::to_string as json_to_string;
use edgerun_json::CompiledTapeKey;
use edgerun_json::JsonValue;
use std::hint::black_box;
use std::time::Duration;
use std::time::Instant;

const CASES: &[(usize, usize)] = &[(1_024, 20_000), (16 * 1_024, 5_000), (256 * 1_024, 400)];

fn main() {
    println!("edgerun-codex-boost benchmark");
    println!("case_bytes,op,iterations,total_ms,ns_per_op,bytes_per_op");

    for &(body_bytes, iterations) in CASES {
        let record = bench_record(body_bytes);
        let archived = archive_record(&record).expect("archive benchmark fixture");
        let text = text_materialize(&record);
        let text_bytes = text.as_bytes().to_vec();
        let json = json_materialize(&record);
        let edgerun_owned = parse_json(&json).expect("parse edgerun_json fixture");
        let edgerun_tape = parse_json_tape(&json).expect("parse edgerun_json tape fixture");
        let edgerun_tape_root = edgerun_tape.root(&json).expect("tape root");
        let edgerun_tape_index = edgerun_tape_root
            .build_object_index()
            .expect("tape object index");
        let body_key = CompiledTapeKey::new("body");

        bench_once(body_bytes, "rkyv_archive", iterations, || {
            let bytes = archive_record(black_box(&record)).expect("archive record");
            black_box(bytes.len())
        });
        bench_once(body_bytes, "rkyv_decode_checked", iterations, || {
            let decoded = decode_record(black_box(&archived)).expect("decode record");
            black_box(decoded.body.len())
        });
        bench_once(body_bytes, "rkyv_access_checked", iterations, || {
            let accessed = access_record(black_box(&archived)).expect("access archived record");
            black_box(&accessed.body);
            black_box(1)
        });
        bench_once(body_bytes, "text_materialize", iterations, || {
            let rendered = text_materialize(black_box(&record));
            black_box(rendered.len())
        });
        bench_once(body_bytes, "text_clone_bytes", iterations, || {
            let bytes = black_box(&text_bytes).clone();
            black_box(bytes.len())
        });
        bench_once(body_bytes, "edgerun_json_owned_lookup", iterations, || {
            let parsed = parse_json(black_box(&json)).expect("parse edgerun_json");
            black_box(parsed.required_str("body").expect("body").len())
        });
        bench_once(
            body_bytes,
            "edgerun_json_owned_lookup_preparsed",
            iterations,
            || {
                black_box(
                    black_box(&edgerun_owned)
                        .required_str("body")
                        .expect("body")
                        .len(),
                )
            },
        );
        bench_once(body_bytes, "edgerun_json_tape_lookup", iterations, || {
            let tape = parse_json_tape(black_box(&json)).expect("parse edgerun_json tape");
            let root = tape.root(black_box(&json)).expect("root");
            black_box(root.required_str("body").expect("body").len())
        });
        bench_once(
            body_bytes,
            "edgerun_json_tape_lookup_preparsed",
            iterations,
            || {
                black_box(
                    black_box(&edgerun_tape_root)
                        .required_str("body")
                        .expect("body")
                        .len(),
                )
            },
        );
        bench_once(
            body_bytes,
            "edgerun_json_tape_indexed_lookup_preparsed",
            iterations,
            || {
                let root = black_box(&edgerun_tape_root);
                let indexed = root.with_index(black_box(&edgerun_tape_index));
                black_box(
                    indexed
                        .get_compiled(black_box(&body_key))
                        .and_then(|value| value.as_str())
                        .map(str::len)
                        .unwrap_or_default(),
                )
            },
        );
    }
}

fn bench_once(body_bytes: usize, name: &str, iterations: usize, mut op: impl FnMut() -> usize) {
    let mut checksum = 0usize;
    for _ in 0..10 {
        checksum ^= op();
    }

    let started = Instant::now();
    for _ in 0..iterations {
        checksum ^= op();
    }
    let elapsed = started.elapsed();
    black_box(checksum);

    print_result(body_bytes, name, iterations, elapsed);
}

fn print_result(body_bytes: usize, name: &str, iterations: usize, elapsed: Duration) {
    let total_ns = elapsed.as_nanos();
    let ns_per_op = total_ns / iterations as u128;
    println!(
        "{body_bytes},{name},{iterations},{:.3},{ns_per_op},{}",
        elapsed.as_secs_f64() * 1000.0,
        body_bytes as u128
    );
}

fn bench_record(body_bytes: usize) -> CodexBoostRecord {
    let mut body = String::with_capacity(body_bytes);
    while body.len() < body_bytes {
        body.push_str(
            "edgerun codex context: command authority lives in committed stream events; ",
        );
    }
    body.truncate(body_bytes);
    CodexBoostRecord::repo_context("AGENTS.md", body)
}

fn text_materialize(record: &CodexBoostRecord) -> String {
    format!(
        "schema_version={}\nkind={:?}\nsource={}\nbody=\n{}",
        record.schema_version, record.kind, record.source, record.body
    )
}

fn json_materialize(record: &CodexBoostRecord) -> String {
    format!(
        "{{\"schema_version\":{},\"kind\":\"{:?}\",\"source\":{},\"body\":{}}}",
        record.schema_version,
        record.kind,
        json_to_string(&JsonValue::from(record.source.as_str())).expect("json source string"),
        json_to_string(&JsonValue::from(record.body.as_str())).expect("json body string")
    )
}
