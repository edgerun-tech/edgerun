fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_unsafe_attr)");
    println!("cargo:rustc-cfg=has_unsafe_attr");
}
