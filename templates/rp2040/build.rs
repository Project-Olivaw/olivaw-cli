//! Puts memory.x on the linker search path. Standard cortex-m-rt plumbing.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR set by cargo"));
    fs::copy("memory.x", out.join("memory.x")).expect("copying memory.x");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
}
