use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use wasmparser::{Validator, Parser, Payload};
use crate::artifact::ArtifactWriter;

pub struct ValidationBench;

impl ValidationBench {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, artifacts: &mut ArtifactWriter) -> Result<()> {
        let samples = [
            "crates/edgerun-wasm-bench/samples/sample_a_empty.wasm",
            "crates/edgerun-wasm-bench/samples/sample_b_cpu_loop.wasm",
            "crates/edgerun-wasm-bench/samples/sample_c_memory_scan.wasm",
            "crates/edgerun-wasm-bench/samples/sample_d_hostcall.wasm",
            "crates/edgerun-wasm-bench/samples/sample_e_payload.wasm",
        ];

        let mut results = String::new();
        results.push_str("{\n");
        results.push_str("  \"validation\": [\n");

        for (i, sample) in samples.iter().enumerate() {
            let wasm = match std::fs::read(sample) {
                Ok(w) => w,
                Err(_) => continue,
            };

            let file_size = wasm.len();
            
            let cold_times = self.benchmark_cold(&wasm, 10)?;
            let warm_times = self.benchmark_warm(&wasm, 100)?;

            results.push_str(&format!("    {{\n"));
            let sample_name = sample.split('/').last().unwrap_or("?");
            results.push_str(&format!("      \"sample\": \"{}\",\n", sample_name));
            results.push_str(",\n");
            results.push_str(&format!("      \"size_bytes\": {},\n", file_size));
            results.push_str(&format!("      \"cold_ms_min\": {},\n", cold_times.0));
            results.push_str(&format!("      \"cold_ms_max\": {},\n", cold_times.1));
            results.push_str(&format!("      \"warm_ms_min\": {},\n", warm_times.0));
            results.push_str(&format!("      \"warm_ms_median\": {},\n", warm_times.1));
            results.push_str(&format!("      \"warm_ms_p90\": {},\n", warm_times.2));
            results.push_str(&format!("      \"warm_ms_p99\": {},\n", warm_times.3));
            results.push_str(&format!("      \"throughput_bytes_per_sec\": {}\n", if warm_times.1 > 0.0 { file_size as f64 / warm_times.1 * 1000.0 } else { 0.0 }));
            results.push_str(&format!("    }}{}\n", if i < samples.len() - 1 { " " } else { "" }));
        }

        results.push_str("  ]\n");
        results.push_str("}\n");

        artifacts.write("validation.json", &results)?;
        Ok(())
    }

    fn benchmark_cold(&self, wasm: &[u8], runs: usize) -> Result<(f64, f64)> {
        let mut times = Vec::with_capacity(runs);
        
        for _ in 0..runs {
            let start = Instant::now();
            let mut validator = Validator::new();
            validator.validate_all(wasm)?;
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            times.push(elapsed);
        }

        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Ok((times.first().copied().unwrap_or(0.0), times.last().copied().unwrap_or(0.0)))
    }

    fn benchmark_warm(&self, wasm: &[u8], runs: usize) -> Result<(f64, f64, f64, f64)> {
        let mut times = Vec::with_capacity(runs);
        
        for _ in 0..runs {
            let start = Instant::now();
            let mut validator = Validator::new();
            validator.validate_all(wasm)?;
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            times.push(elapsed);
        }

        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let n = times.len();
        let min = times.first().copied().unwrap_or(0.0);
        let median = times[n / 2];
        let p90 = times[(n as f64 * 0.9) as usize];
        let p99 = times[(n as f64 * 0.99) as usize].min(*times.last().unwrap_or(&median));

        Ok((min, median, p90, p99))
    }
}