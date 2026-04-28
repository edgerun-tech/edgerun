use edgerun_json::{
    from_slice as edgerun_from_slice, from_str as edgerun_from_str, parse_json,
    parse_json_borrowed, parse_json_tape, to_string as edgerun_to_string, to_vec as edgerun_to_vec,
    JsonValue, Map,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

const ITERS_SMALL: usize = 20_000;
const ITERS_MEDIUM: usize = 5_000;
const ITERS_LARGE: usize = 500;

fn small_json() -> &'static str {
    r#"{"id":42,"name":"test","active":true}"#
}

fn medium_json() -> &'static str {
    r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#
}

fn wide_json() -> String {
    let mut pairs = Vec::with_capacity(100);
    for i in 0..100 {
        pairs.push(format!("\"key{}\":{}", i, i * 7));
    }
    format!("{{{}}}", pairs.join(","))
}

fn large_array_json() -> String {
    let obj = r#"{"id":1,"name":"item","v":3.14}"#;
    let mut s = String::with_capacity(obj.len() * 1000 + 2);
    s.push('[');
    for i in 0..1000 {
        if i > 0 {
            s.push(',');
        }
        s.push_str(
            &obj.replace("1", &i.to_string())
                .replace("3.14", &format!("{:.2}", i as f64 * 0.01)),
        );
    }
    s.push(']');
    s
}

fn sample_value() -> JsonValue {
    let mut metadata = JsonValue::empty_object();
    metadata.push_field("created", 1_234_567_890i64);
    metadata.push_field("updated", 9_876_543_210i64);

    let mut user = JsonValue::empty_object();
    user.push_field("id", 12_345i64);
    user.push_field("name", "John Doe");
    user.push_field("email", "john@example.com");
    user.push_field("metadata", metadata);

    let mut root = JsonValue::empty_object();
    root.push_field("user", user);
    root.push_field("status", "active");
    root.push_field("count", 100i64);
    root
}

fn bench<F>(name: &str, iterations: usize, mut f: F) -> Duration
where
    F: FnMut(),
{
    f();
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iterations as u32;
    println!("{name:<36} {:>10} ns/iter", per_iter.as_nanos());
    per_iter
}

fn compare<F, G>(name: &str, iterations: usize, edgerun: F, upstream: G)
where
    F: FnMut(),
    G: FnMut(),
{
    let edgerun_time = bench(&format!("{name} edgerun"), iterations, edgerun);
    let upstream_time = bench(&format!("{name} serde_json"), iterations, upstream);
    let ratio = upstream_time.as_secs_f64() / edgerun_time.as_secs_f64();
    println!("{name:<36} {:>10.2}x\n", ratio);
}

fn main() {
    println!("edgerun-json custom benchmark");
    println!("lower ns/iter is better; ratio is serde_json / edgerun");
    println!();

    let small = small_json();
    let medium = medium_json();
    let wide = wide_json();
    let large = large_array_json();
    let value = sample_value();
    let upstream_value: serde_json_upstream::Value = serde_json_upstream::from_str(medium).unwrap();

    compare(
        "parse small",
        ITERS_SMALL,
        || {
            black_box(parse_json(black_box(small)).unwrap());
        },
        || {
            black_box(
                serde_json_upstream::from_str::<serde_json_upstream::Value>(black_box(small))
                    .unwrap(),
            );
        },
    );

    compare(
        "parse medium",
        ITERS_MEDIUM,
        || {
            black_box(parse_json(black_box(medium)).unwrap());
        },
        || {
            black_box(
                serde_json_upstream::from_str::<serde_json_upstream::Value>(black_box(medium))
                    .unwrap(),
            );
        },
    );

    compare(
        "parse wide",
        ITERS_MEDIUM,
        || {
            black_box(parse_json(black_box(&wide)).unwrap());
        },
        || {
            black_box(
                serde_json_upstream::from_str::<serde_json_upstream::Value>(black_box(&wide))
                    .unwrap(),
            );
        },
    );

    compare(
        "parse large array",
        ITERS_LARGE,
        || {
            black_box(parse_json(black_box(&large)).unwrap());
        },
        || {
            black_box(
                serde_json_upstream::from_str::<serde_json_upstream::Value>(black_box(&large))
                    .unwrap(),
            );
        },
    );

    bench("borrowed parse medium", ITERS_MEDIUM, || {
        black_box(parse_json_borrowed(black_box(medium)).unwrap());
    });
    bench("tape parse medium", ITERS_MEDIUM, || {
        black_box(parse_json_tape(black_box(medium)).unwrap());
    });
    println!();

    compare(
        "from_slice medium",
        ITERS_MEDIUM,
        || {
            black_box(edgerun_from_slice::<JsonValue>(black_box(medium.as_bytes())).unwrap());
        },
        || {
            black_box(
                serde_json_upstream::from_slice::<serde_json_upstream::Value>(black_box(
                    medium.as_bytes(),
                ))
                .unwrap(),
            );
        },
    );

    compare(
        "serialize medium",
        ITERS_MEDIUM,
        || {
            black_box(edgerun_to_string(black_box(&value)).unwrap());
        },
        || {
            black_box(serde_json_upstream::to_string(black_box(&upstream_value)).unwrap());
        },
    );

    compare(
        "to_vec medium",
        ITERS_MEDIUM,
        || {
            black_box(edgerun_to_vec(black_box(&value)).unwrap());
        },
        || {
            black_box(serde_json_upstream::to_vec(black_box(&upstream_value)).unwrap());
        },
    );

    compare(
        "roundtrip medium",
        ITERS_MEDIUM,
        || {
            let parsed: JsonValue = edgerun_from_str(black_box(medium)).unwrap();
            black_box(edgerun_to_string(&parsed).unwrap());
        },
        || {
            let parsed: serde_json_upstream::Value =
                serde_json_upstream::from_str(black_box(medium)).unwrap();
            black_box(serde_json_upstream::to_string(&parsed).unwrap());
        },
    );
}
