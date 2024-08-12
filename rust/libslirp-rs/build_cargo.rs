use bindgen;
use std::env;
use std::path::PathBuf;

pub fn main() {
    println!("cargo:rerun-if-changed=bindings/wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");

    let bindings = bindgen::Builder::default()
        .header("bindings/wrapper.h")
        .generate()
        .expect("Unable to generate bindings");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("src")
        .join("libslirp_sys")
        .join(target_os);

    std::fs::create_dir_all(&out_dir).expect("Failed to create output directory");

    let out_path = out_dir.join("bindings.rs");
    bindings.write_to_file(out_path).expect("Couldn't write bindings!");
}
