// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use edgerun_device_cap_host::benchmark::{run_benchmark_suite, BenchmarkProfile};

fn parse_profile(arg: Option<&str>) -> BenchmarkProfile {
    match arg.unwrap_or("edge-standard") {
        "router-lite" => BenchmarkProfile::RouterLite,
        "edge-performance" => BenchmarkProfile::EdgePerformance,
        _ => BenchmarkProfile::EdgeStandard,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let profile = parse_profile(args.next().as_deref());
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("out/bench"));

    match run_benchmark_suite(profile, &output) {
        Ok(path) => {
            println!("benchmark report written: {}", path.display());
        }
        Err(err) => {
            eprintln!("benchmark run failed: {err}");
            std::process::exit(1);
        }
    }
}
