fn main() {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    println!("cargo:rustc-link-lib=c");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/imports.c");
}
