fn main() {
    //cc::Build::new()
    //    .file("src/imports.c")
    //    .compile("kloroutines");

    println!("cargo:rustc-link-lib=c");
    //println!("cargo:rustc-link-lib=kloroutines");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/imports.c");
}
