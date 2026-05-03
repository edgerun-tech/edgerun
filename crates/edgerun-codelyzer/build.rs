fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .out_dir("src/generated")
        .compile_protos(&["proto/codeanalyzer.proto"], &["proto/"])?;
    Ok(())
}
