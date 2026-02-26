// Copyright 2026 The Android Open Source Project

//! # Intentions Codegen
//!
//! This binary parses source files to find methods annotated with `/// STEP:`
//! comments. It generates "glue" code that register these steps.

mod kotlin;
mod rust;
mod utils;

use std::{env, path::Path};

use kotlin::process_kotlin_file;
use rust::process_rust_file;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: verify_codegen <input_file> <output_file>");
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    if let Some(ext) = input_path.extension() {
        if ext == "rs" {
            process_rust_file(input_path, output_path);
        } else if ext == "kt" {
            process_kotlin_file(input_path, output_path);
        } else {
            eprintln!("Unsupported file extension: {:?}", ext);
            std::process::exit(1);
        }
    } else {
        eprintln!("No file extension found");
        std::process::exit(1);
    }
}
