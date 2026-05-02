use anyhow::Result;
use std::time::Instant;
use wasmtime::{
    Engine, Module, Store, Linker, 
    TypedFunc,
};
use crate::artifact::ArtifactWriter;

pub struct RuntimeBench;

impl RuntimeBench {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, artifacts: &mut ArtifactWriter) -> Result<()> {
        let samples = [
            "crates/edgerun-wasm-bench/samples/sample_a_empty.wasm",
            "crates/edgerun-wasm-bench/samples/sample_b_cpu_loop.wasm",
            "crates/edgerun-wasm-bench/samples/sample_c_memory_scan.wasm",
        ];

        let mut results = String::new();
        results.push_str("{\n");
        results.push_str("  \"runtime\": [\n");

        for (i, sample) in samples.iter().enumerate() {
            let wasm = match std::fs::read(sample) {
                Ok(w) => w,
                Err(_) => continue,
            };

            let file_size = wasm.len();
            let engine = Engine::default();
            
            let cold = self.benchmark_cold(&engine, &wasm)?;
            let warm = self.benchmark_warm(&engine, &wasm)?;
            let hot = self.benchmark_hot(&engine, &wasm)?;

            results.push_str(&format!("    {{\n"));
            let sample_name = sample.split('/').last().unwrap_or("?");
            results.push_str(&format!("      \"sample\": \"{}\",\n", sample_name));
            results.push_str(&format!("      \"size_bytes\": {},\n", file_size));
            results.push_str(&format!("      \"cold_read_ms\": {:.3},\n", cold.0));
            results.push_str(&format!("      \"cold_compile_ms\": {:.3},\n", cold.1));
            results.push_str(&format!("      \"cold_instantiate_ms\": {:.3},\n", cold.2));
            results.push_str(&format!("      \"cold_first_call_ms\": {:.3},\n", cold.3));
            results.push_str(&format!("      \"warm_instantiate_ms_median\": {:.3},\n", warm.0));
            results.push_str(&format!("      \"warm_call_ms_median\": {:.3},\n", warm.1));
            results.push_str(&format!("      \"hot_call_ms_median\": {:.3},\n", hot.0));
            results.push_str(&format!("      \"hot_calls_per_sec\": {:.0}\n", hot.1));
            results.push_str(&format!("    }}{}\n", if i < samples.len() - 1 { " " } else { "" }));
        }

        results.push_str("  ]\n");
        results.push_str("}\n");

        artifacts.write("runtime-wasmtime.json", &results)?;
        Ok(())
    }

    fn benchmark_cold(&self, engine: &Engine, wasm: &[u8]) -> Result<(f64, f64, f64, f64)> {
        let t0 = Instant::now();
        let _read: Vec<u8> = wasm.to_vec();
        let read_ms = t0.elapsed().as_secs_f64() * 1000.0;
        
        let t1 = Instant::now();
        let module = Module::new(engine, wasm)?;
        let compile_ms = t1.elapsed().as_secs_f64() * 1000.0;
        
        let t2 = Instant::now();
        let mut store = Store::new(engine, ());
        let instantiate_ms = t2.elapsed().as_secs_f64() * 1000.0;
        
        let t3 = Instant::now();
        let linker = Linker::new(engine);
        let instance = linker.instantiate(&mut store, &module)?;
        let func: TypedFunc<(i32, i32), i32> = instance.get_typed_func(&mut store, "run")?;
        let _ = func.call(&mut store, (0, 0))?;
        let first_call_ms = t3.elapsed().as_secs_f64() * 1000.0;
        
        Ok((read_ms, compile_ms, instantiate_ms, first_call_ms))
    }

    fn benchmark_warm(&self, engine: &Engine, wasm: &[u8]) -> Result<(f64, f64)> {
        let module = Module::new(engine, wasm)?;
        
        let mut instantiate_times = Vec::with_capacity(100);
        let mut call_times = Vec::with_capacity(100);
        
        for _ in 0..100 {
            let t0 = Instant::now();
            let mut store = Store::new(engine, ());
            let linker = Linker::new(engine);
            let _instance = linker.instantiate(&mut store, &module)?;
            instantiate_times.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        
        let mut store = Store::new(engine, ());
        let linker = Linker::new(engine);
        let instance = linker.instantiate(&mut store, &module)?;
        let func: TypedFunc<(i32, i32), i32> = instance.get_typed_func(&mut store, "run")?;
        
        for _ in 0..100 {
            let t0 = Instant::now();
            let _ = func.call(&mut store, (0, 0))?;
            call_times.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        
        instantiate_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        call_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let n = instantiate_times.len();
        Ok((instantiate_times[n/2], call_times[n/2]))
    }

    fn benchmark_hot(&self, engine: &Engine, wasm: &[u8]) -> Result<(f64, f64)> {
        let module = Module::new(engine, wasm)?;
        let mut store = Store::new(engine, ());
        let linker = Linker::new(engine);
        let instance = linker.instantiate(&mut store, &module)?;
        let func: TypedFunc<(i32, i32), i32> = instance.get_typed_func(&mut store, "run")?;
        
        let iterations = 10000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = func.call(&mut store, (0, 0))?;
        }
        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let per_call = total_ms / iterations as f64;
        let calls_per_sec = 1000.0 / per_call;
        
        Ok((per_call, calls_per_sec))
    }
}