use cmake;
use cxx_build;

fn main() {

    // #[cfg(target_arch = "x86_64")]
    // {
    //     cxx_build::bridge("src/slang/mod.rs")
    //         .file("src/slang/slang.cc")
    //         .std("c++14")
    //         .compile("poisson_renderer");
    //
    //     let mut dst = cmake::build("external/slang");
    //     dst.push("lib");
    //
    //     println!("cargo:rustc-link-search=native={}", dst.display());
    //     println!("cargo:rustc-link-lib=dylib=slang");
    //
    //     println!("cargo:rerun-if-changed=src/bin/main.rs");
    //     println!("cargo:rerun-if-changed=src/slang/slang.cpp");
    //     println!("cargo:rerun-if-changed=src/slang/slang.h");
    // }
}
