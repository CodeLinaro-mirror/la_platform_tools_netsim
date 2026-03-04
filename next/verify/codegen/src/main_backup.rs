// Copyright 2026 The Android Open Source Project

//! # Intentions Codegen
//!
//! This binary parses source files to find methods annotated with `/// STEP:`
//! comments. It generates "glue" code that register these steps.

use std::{env, fs, path::Path};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use syn::{visit::Visit, Attribute, FnArg, ImplItem, ItemImpl, Pat, Type};

/// Visitor for Rust source files
struct StepVisitor {
    glue_code: Vec<TokenStream>,
    registration_code: Vec<TokenStream>,
    regex_matcher: Regex,
    struct_type: Option<Type>,
}

impl StepVisitor {
    fn new() -> Self {
        Self {
            glue_code: Vec::new(),
            registration_code: Vec::new(),
            regex_matcher: Regex::new(r"STEP:\s*(Given|When|Then|And|Before|After)(?:\s+(.*))?")
                .unwrap(),
            struct_type: None,
        }
    }
}

impl<'ast> Visit<'ast> for StepVisitor {
    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let current_struct_type = &*node.self_ty;

        // Iterate over items in the impl block
        for item in &node.items {
            if let ImplItem::Fn(m) = item {
                self.process_item(&m.attrs, &m.sig.ident, &m.sig.inputs, Some(current_struct_type));
            }
        }

        // Continue visiting children
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.process_item(&node.attrs, &node.sig.ident, &node.sig.inputs, None);
        syn::visit::visit_item_fn(self, node);
    }
}

impl StepVisitor {
    fn process_item(
        &mut self,
        attrs: &[Attribute],
        method_name: &syn::Ident,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        explicit_struct_type: Option<&Type>,
    ) {
        for attr in attrs {
            let comment = match parse_doc_comment(attr) {
                Some(c) => c,
                None => continue,
            };

            let caps = match self.regex_matcher.captures(comment.trim()) {
                Some(c) => c,
                None => continue,
            };

            // Capture struct type if not yet found
            if self.struct_type.is_none() {
                if let Some(st) = explicit_struct_type {
                    self.struct_type = Some(st.clone());
                } else if let Some(FnArg::Typed(pat_type)) = inputs.first() {
                    // Expecting &mut World
                    if let Type::Reference(type_ref) = &*pat_type.ty {
                        if type_ref.mutability.is_some() {
                            self.struct_type = Some(*type_ref.elem.clone());
                        }
                    }
                }
            }

            let keyword = caps.get(1).unwrap().as_str();
            let pattern = caps.get(2).map_or("", |m| m.as_str());
            let wrapper_name = format_ident!("{}_wrapper", method_name);

            self.generate_glue(
                keyword,
                pattern,
                method_name,
                wrapper_name,
                inputs,
                explicit_struct_type,
            );
        }
    }
}

impl StepVisitor {
    fn generate_glue(
        &mut self,
        keyword: &str,
        pattern: &str,
        method_name: &syn::Ident,
        wrapper_name: syn::Ident,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        struct_type: Option<&Type>,
    ) {
        // Generate argument parsing logic
        let mut args_parsing = Vec::new();
        let mut method_call_args = Vec::new();
        let mut arg_index = 0;

        // If this is a method (struct_type is Some), the first arg is implicit self
        // (receiver). If this is a free function (struct_type is None), the
        // first arg MUST be &mut World.

        // For free functions, we pass 'w' as first arg to the call.
        if struct_type.is_none() {
            method_call_args.push(quote! { w });
        }

        for (i, input) in inputs.iter().enumerate() {
            match input {
                FnArg::Receiver(_) => {} // skip self
                FnArg::Typed(pat_type) => {
                    // For free function, skip first arg (World)
                    if struct_type.is_none() && i == 0 {
                        continue;
                    }

                    if let Pat::Ident(_) = &*pat_type.pat {
                        let ty = &pat_type.ty;
                        let arg_ident = format_ident!("arg{}", arg_index);
                        let arg_idx_lit = syn::Index::from(arg_index);

                        // Check if type is String or DataTable
                        let mut is_string = false;
                        let mut is_data_table = false;

                        if let Type::Path(tp) = &**ty {
                            // Check for String or DataTable (checking last segment to support
                            // qualified paths)
                            if let Some(seg) = tp.path.segments.last() {
                                if seg.ident == "String" {
                                    is_string = true;
                                } else if seg.ident == "DataTable" {
                                    is_data_table = true;
                                }
                            }
                        }

                        if is_data_table {
                            // We don't parse it from args, we get it from ctx
                            args_parsing.push(quote! {
                                 let #arg_ident = ctx.table.clone().expect("Step expects a Data Table but none was provided");
                             });
                        } else if is_string {
                            args_parsing.push(quote! {
                                let #arg_ident = args[#arg_idx_lit].clone();
                            });
                            arg_index += 1;
                        } else {
                            let err_msg = format!("Failed to parse argument {}", arg_index);
                            args_parsing.push(quote! {
                                let #arg_ident = args[#arg_idx_lit].parse::<#ty>().expect(#err_msg);
                            });
                            arg_index += 1;
                        }
                        method_call_args.push(quote! { #arg_ident });
                    }
                }
            }
        }

        // We need the world type for the wrapper signature.
        let world_type = if let Some(st) = struct_type {
            st.clone()
        } else {
            // Must have found it by now
            self.struct_type.clone().expect("Could not infer World type from free function")
        };

        let call_stmt = if struct_type.is_some() {
            quote! { w.#method_name(#(#method_call_args),*).await; }
        } else {
            quote! { #method_name(#(#method_call_args),*).await; }
        };

        let glue = quote! {
            fn #wrapper_name(w: &mut #world_type, #[allow(unused_variables)] args: Vec<String>, #[allow(unused_variables)] ctx: ::features::StepContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
                Box::pin(async move {
                    #(#args_parsing)*
                    #call_stmt
                })
            }
        };

        self.glue_code.push(glue);

        let keyword_ident = format_ident!("{}", keyword.to_lowercase());

        let reg_call = if keyword == "Before" || keyword == "After" {
            quote! {
                c.#keyword_ident(#wrapper_name);
            }
        } else {
            let pattern_lit = proc_macro2::Literal::string(pattern);
            quote! {
                c.#keyword_ident(#pattern_lit, #wrapper_name);
            }
        };

        self.registration_code.push(reg_call);
    }
}

fn parse_doc_comment(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("doc") {
        return None;
    }
    let syn::Meta::NameValue(meta) = &attr.meta else {
        return None;
    };
    let syn::Expr::Lit(expr_lit) = &meta.value else {
        return None;
    };
    let syn::Lit::Str(lit_str) = &expr_lit.lit else {
        return None;
    };

    Some(lit_str.value())
}

fn process_rust_file(input_path: &Path, output_path: &Path) {
    let content = fs::read_to_string(input_path).expect("Failed to read input file");
    let syntax = syn::parse_file(&content).expect("Failed to parse Rust code");

    let mut visitor = StepVisitor::new();
    visitor.visit_file(&syntax);

    let glue_code = &visitor.glue_code;
    let registration_code = &visitor.registration_code;

    let register_fn = if let Some(struct_type) = &visitor.struct_type {
        quote! {
            pub fn register_steps(c: &mut ::features::Features<#struct_type>) {
                #(#registration_code)*
            }
        }
    } else {
        quote! {
            pub fn register_steps<W>(c: &mut ::features::Features<W>) {}
        }
    };

    let generated_rs = quote! {
        // Generated by codegen
        #(#glue_code)*
        #register_fn
    };

    fs::write(output_path, generated_rs.to_string()).expect("Failed to write output file");
}

fn process_kotlin_file(input_path: &Path, output_path: &Path) {
    let content = fs::read_to_string(input_path).expect("Failed to read input file");
    let regex_matcher = Regex::new(r"/// STEP:\s*(.*)").unwrap();
    let func_matcher = Regex::new(r"fun\s+(\w+)").unwrap();

    let mut registration_lines = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Naively scan for comments followed by fun definition
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = regex_matcher.captures(line.trim()) {
            let pattern_raw = caps.get(1).map_or("", |m| m.as_str());
            let pattern = pattern_raw.replace("\\", "\\\\");

            // Look ahead for function name
            // Allow up to 5 lines of gap (annotations, etc)
            for j in 1..=5 {
                if i + j < lines.len() {
                    let next_line = lines[i + j].trim();
                    if let Some(func_caps) = func_matcher.captures(next_line) {
                        let func_name = func_caps.get(1).unwrap().as_str();

                        // We assume the function signature is (List<String>) -> Unit or (Context,
                        // List<String>) -> Unit For simplicity in this
                        // codegen, we assume we invoke it. However, we need
                        // to know the Class name. Limitation: This naive
                        // parser doesn't know the package/class name easily without more parsing.
                        // Assuming the output file is generated in the SAME package or has imports.
                        // Actually, it's better if we generate a `StepLoader` object that calls
                        // these functions. But these functions need to be
                        // importable.

                        // For now, let's assume the user will supply the class instance or we
                        // generate calls to the functions if they are
                        // top-level. If they are members of a class, this is harder.

                        // Let's assume top-level functions or object methods.
                        // If the input file is `WifiSteps.kt`, we might assume a `WifiSteps`
                        // object?

                        // Strategy: Use a specific convention. e.g. "WifiSteps".funcName

                        // If we are just generating specific registrations for THIS file

                        // Kotlin dispatch: registry.register("pattern") { args -> funcName(args) }

                        // We need to know if it takes context.
                        // Let's rely on the function taking (Context, List<String>) or just
                        // (List<String>). Hack: Inspect the line with `fun`
                        // for `Context`.

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
        .unwrap_or("com.android.netsim.ntest");

    // File name -> Class Name (WifiSteps.kt -> WifiStepsLoader)
    let file_stem = input_path.file_stem().unwrap().to_string_lossy();
    let loader_name = format!("{}Loader", file_stem);

    let mut output = String::new();
    output.push_str(&format!("package {}\n\n", package_name));
    output.push_str("import android.content.Context\n");
    // output.push_str("import com.android.netsim.ntest.StepRegistry\n\n"); //
    // Assumed available

    output.push_str(&format!("object {} {{\n", loader_name));
    output.push_str("    fun loadSteps(registry: StepRegistry, context: Context) {\n");
    for line in registration_lines {
        output.push_str(&format!("{}\n", line));
    }
    output.push_str("    }\n");
    output.push_str("}\n");

    fs::write(output_path, output).expect("Failed to write output file");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: intentions_codegen <input_file> <output_file>");
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
