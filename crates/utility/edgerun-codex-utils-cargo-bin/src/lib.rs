use std::path::PathBuf;

pub fn cargo_bin(name: &str) -> std::io::Result<PathBuf> {
    let key = format!("CARGO_BIN_EXE_{}", name.replace('-', "_"));
    if let Some(path) = std::env::var_os(&key) {
        return Ok(PathBuf::from(path));
    }

    let mut path = std::env::current_exe()?;
    while path.pop() {
        let candidate = path.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
        let candidate_exe = path.join(format!("{name}.exe"));
        if candidate_exe.exists() {
            return Ok(candidate_exe);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("cargo test binary not found: {name}"),
    ))
}

pub fn repo_root() -> std::io::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join(".git").exists()
            || dir.join("Cargo.toml").exists() && dir.join("patch").exists()
        {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "repository root not found",
            ));
        }
    }
}
