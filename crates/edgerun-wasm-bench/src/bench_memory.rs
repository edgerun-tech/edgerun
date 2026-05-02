use anyhow::Result;
use std::fs;
use std::process::Command;
use crate::artifact::ArtifactWriter;

pub struct MemoryBench;

impl MemoryBench {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, artifacts: &mut ArtifactWriter) -> Result<()> {
        let rss_before = self.get_rss()?;
        let threads_before = self.get_thread_count()?;

        let empty_wasm = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x04, 0x01, 0x80, 0x80, 0x80, 0x00, 0x00,
            0x03, 0x02, 0x00, 0x00, 0x05, 0x07, 0x01, 0x02,
            0x68, 0x75, 0x6e, 0x00, 0x00, 0x0a, 0x06, 0x01,
            0x04, 0x00, 0x00, 0x00, 0x00, 0x0b
        ];

        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::new(&engine, &empty_wasm)?;
        let mut store = wasmtime::Store::new(&engine, ());
        let linker = wasmtime::Linker::new(&engine);
        let _instance = linker.instantiate(&mut store, &module)?;

        let rss_after = self.get_rss()?;
        let threads_after = self.get_thread_count()?;

        let mut results = String::new();
        results.push_str("{\n");
        results.push_str("  \"footprint\": {\n");
        results.push_str(&format!("    \"rss_before_kb\": {},\n", rss_before));
        results.push_str(&format!("    \"rss_after_load_kb\": {},\n", rss_after));
        results.push_str(&format!("    \"rss_delta_kb\": {},\n", rss_after.saturating_sub(rss_before)));
        results.push_str(&format!("    \"threads_before\": {},\n", threads_before));
        results.push_str(&format!("    \"threads_after\": {},\n", threads_after));
        results.push_str(&format!("    \"threads_delta\": {}\n", threads_after.saturating_sub(threads_before)));
        results.push_str("  }\n");
        results.push_str("}\n");

        artifacts.write("footprint.json", &results)?;
        Ok(())
    }

    fn get_rss(&self) -> Result<u64> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("cat /proc/self/status | grep VmRSS | awk '{print $2}'")
            .output()?;
        
        let value = String::from_utf8_lossy(&output.stdout);
        Ok(value.trim().parse().unwrap_or(0))
    }

    fn get_thread_count(&self) -> Result<u32> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("cat /proc/self/status | grep Threads | awk '{print $2}'")
            .output()?;
        
        let value = String::from_utf8_lossy(&output.stdout);
        Ok(value.trim().parse().unwrap_or(1))
    }
}