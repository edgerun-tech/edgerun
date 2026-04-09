//! Build script — link TSS2 ESAPI libraries.

fn main() {
    println!("cargo:rustc-link-lib=tss2-esys");
    println!("cargo:rustc-link-lib=tss2-tctildr");
    println!("cargo:rustc-link-lib=tss2-mu");
    println!("cargo:rustc-link-lib=tss2-sys");
}
