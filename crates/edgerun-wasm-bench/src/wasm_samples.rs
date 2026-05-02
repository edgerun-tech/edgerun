use anyhow::Result;
use std::path::Path;
use std::fs;
use std::process::Command;

pub fn ensure_samples(dir: &str) -> Result<()> {
    let dir = Path::new(dir);
    fs::create_dir_all(dir)?;

    let wasm_samples = [
        ("sample_a_empty.wasm", vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x04, 0x01, 0x80, 0x80, 0x80, 0x00, 0x00,
            0x03, 0x02, 0x00, 0x00,
            0x05, 0x03, 0x01, 0x02, 0x68, 0x00,
            0x07, 0x03, 0x01, 0x02, 0x72, 0x75, 0x6e, 0x00, 0x00,
            0x09, 0x03, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x0b,
        ]),
    ];

    for (name, bytes) in wasm_samples {
        let path = dir.join(name);
        fs::write(&path, &bytes)?;
    }

    let has_wat2wasm = Command::new("wat2wasm").output().is_ok();
    if has_wat2wasm {
        generate_wat_samples(dir)?;
    }

    Ok(())
}

fn generate_wat_samples(dir: &Path) -> Result<()> {
    let samples: Vec<(&str, &str)> = vec![
        ("sample_b_cpu_loop.wat", r#"
(module
  (func $loop (export "run") (param i32 i32) (result i32)
    (local i32)
    (i32.const 0)
    (local.set 1)
    (block (br_if 1 (i32.eq (local.get 1) (i32.const 100000))))
    (local.get 1)
    (i32.const 1)
    (i32.add)
    (local.set 1)
    (br 0)
    (local.get 1))
)"#),
        ("sample_c_memory_scan.wat", r#"
(module
  (memory 1
  (export "run") (param i32 i32) (result i32)
    (i32.const 0)))
)"#),
    ];

    for (name, wat) in samples {
        let wat_path = dir.join(name);
        let wasm_path = dir.join(name.strip_suffix(".wat").unwrap_or(name)).with_extension("wasm");
        
        fs::write(&wat_path, wat)?;
        let output = Command::new("wat2wasm")
            .arg(&wat_path)
            .arg("-o")
            .arg(&wasm_path)
            .output()?;
        
        if !output.status.success() {
            fs::remove_file(&wat_path).ok();
        }
    }

    Ok(())
}