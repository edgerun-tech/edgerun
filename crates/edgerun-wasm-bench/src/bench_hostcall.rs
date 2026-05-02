use anyhow::Result;
use std::time::Instant;
use std::sync::{Arc, Mutex};
use wasmtime::{
    Engine, Module, Store, Linker, 
    TypedFunc, Caller,
};
use crate::artifact::ArtifactWriter;

pub struct HostcallBench;

impl HostcallBench {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, artifacts: &mut ArtifactWriter) -> Result<()> {
        let empty_wasm = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x04, 0x01, 0x80, 0x80, 0x80, 0x00, 0x00,
            0x03, 0x02, 0x00, 0x00, 0x05, 0x07, 0x01, 0x02,
            0x68, 0x75, 0x6e, 0x00, 0x00, 0x0a, 0x06, 0x01,
            0x04, 0x00, 0x00, 0x00, 0x00, 0x0b
        ];

        let engine = Engine::default();
        let module = Module::new(&engine, &empty_wasm)?;

        let call_counts = vec![1, 10, 100, 1000];
        
        let mut results = String::new();
        results.push_str("{\n");
        results.push_str("  \"hostcall\": [\n");

        for (i, count) in call_counts.iter().enumerate() {
            let times = self.benchmark_hostcalls(&engine, &module, *count)?;
            
            results.push_str(&format!("    {{\n"));
            results.push_str(&format!("      \"call_count\": {},\n", count));
            results.push_str(&format!("      \"total_ms\": {:.3},\n", times.0));
            results.push_str(&format!("      \"per_call_median_us\": {:.1},\n", times.1));
            results.push_str(&format!("      \"per_call_p90_us\": {:.1},\n", times.2));
            results.push_str(&format!("      \"per_call_p99_us\": {:.1}\n", times.3));
            results.push_str(&format!("    }}{}\n", if i < call_counts.len() - 1 { " " } else { "" }));
        }

        results.push_str("  ]\n");
        results.push_str("}\n");

        artifacts.write("hostcall.json", &results)?;
        Ok(())
    }

    fn benchmark_hostcalls(&self, engine: &Engine, module: &Module, count: usize) -> Result<(f64, f64, f64, f64)> {
        #[derive(Clone)]
        struct HostState {
            call_count: Arc<Mutex<usize>>,
        }

        let host = HostState {
            call_count: Arc::new(Mutex::new(0)),
        };

        let mut store = Store::new(engine, host.clone());
        let mut linker = Linker::new(engine);
        
        let counter = host.call_count.clone();
        linker.func_wrap(
            "env", 
            "noop", 
            move |_caller: Caller<'_, HostState>| {
                let mut c = counter.lock().unwrap();
                *c += 1;
            }
        )?;
        
        let instance = linker.instantiate(&mut store, module)?;
        let func: TypedFunc<(), ()> = instance.get_typed_func(&mut store, "run")?;

        let mut per_call_times = Vec::with_capacity(count);
        
        for _ in 0..count {
            *host.call_count.lock().unwrap() = 0;
            let start = Instant::now();
            func.call(&mut store, ())?;
            per_call_times.push(start.elapsed().as_secs_f64() * 1_000_000.0);
        }

        per_call_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let n = per_call_times.len();
        let total = per_call_times.iter().sum::<f64>();
        let median = per_call_times[n/2];
        let p90 = per_call_times[(n as f64 * 0.9) as usize];
        let p99 = per_call_times.last().copied().unwrap_or(median);

        Ok((total, median, p90, p99))
    }
}