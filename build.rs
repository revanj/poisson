use cmake;
use cxx_build;

fn main() {
    cxx_build::bridge("src/bin/main.rs")
        .file("src/blobstore.cc")
        .std("c++14")
        .compile("rust-renderer");

    let mut dst = cmake::build("external/slang");
    dst.push("lib");

    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=dylib=slang");
    //
    // println!("cargo:rerun-if-changed=src/bin/main.rs");
    // println!("cargo:rerun-if-changed=src/blobstore.cc");
    // println!("cargo:rerun-if-changed=include/blobstore.h");
}
