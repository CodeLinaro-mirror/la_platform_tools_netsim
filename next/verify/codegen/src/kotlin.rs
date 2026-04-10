// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path};

use regex::Regex;

pub fn process_kotlin_file(input_path: &Path, output_path: &Path) {
    let content = fs::read_to_string(input_path).expect("Failed to read input file");
    let regex_matcher = Regex::new(r"/// STEP:\s*(.*)").unwrap();
    let func_matcher = Regex::new(r"fun\s+(\w+)").unwrap();

    let mut registration_lines = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Naively scan for comments followed by fun definition
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = regex_matcher.captures(line.trim()) {
            let pattern_raw = caps.get(1).map_or("", |m| m.as_str());
            let pattern = pattern_raw.replace("\\", "\\\\").replace("\"", "\\\"");

            // Look ahead for function name
            // Allow up to 5 lines of gap (annotations, etc)
            for j in 1..=5 {
                if i + j < lines.len() {
                    let next_line = lines[i + j].trim();
                    if let Some(func_caps) = func_matcher.captures(next_line) {
                        let func_name = func_caps.get(1).unwrap().as_str();
                        let has_context = next_line.contains("Context");

                        let call = if has_context {
                            format!("{}(context, args)", func_name)
                        } else {
                            format!("{}(args)", func_name)
                        };

                        registration_lines.push(format!(
                            "        registry.register(\"{}\") {{ args -> {} }}",
                            pattern, call
                        ));
                        break;
                    }
                }
            }
        }
    }

    // Generate Kotlin Output
    // We need to infer package name from input file content
    let package_regex = Regex::new(r"package\s+([\w\.]+)").unwrap();
    let package_name = package_regex
        .captures(&content)
        .map(|c| c.get(1).unwrap().as_str())
        .unwrap_or("com.android.verify.vbs");

    // File name -> Class Name (WifiSteps.kt -> WifiStepsLoader)
    let file_stem = input_path.file_stem().unwrap().to_string_lossy();
    let loader_name = format!("{}Loader", file_stem);

    let mut output = String::new();
    output.push_str(&format!("package {}\n\n", package_name));
    output.push_str("import android.content.Context\n");
    output.push_str("import com.android.verify.core.StepRegistry\n\n");

    output.push_str(&format!("object {} {{\n", loader_name));
    output.push_str("    fun loadSteps(registry: StepRegistry, context: Context) {\n");
    for line in registration_lines {
        output.push_str(&format!("{}\n", line));
    }
    output.push_str("    }\n");
    output.push_str("}\n");

    fs::write(output_path, output).expect("Failed to write output file");
}
