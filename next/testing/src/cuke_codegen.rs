// Copyright 2026 The Android Open Source Project

//! # Cuke Codegen
//!
//! This binary parses Rust source files to find methods annotated with `///
//! Cuke:` doc comments. It generates "glue" code that:
//! 1. wrappers around the async methods to handle argument parsing from regex
//!    captures.
//! 2. registers these wrapper functions with the Cuke engine.
//!
//! ## Usage
//!
//! Annotate your World struct methods with:
//! ```rust
//! /// Cuke: <Keyword> <Regex>
//! async fn my_step(&mut self, arg: i32) { ... }
//! ```
//!
//! Supported keywords: Given, When, Then.
//! Supported argument types: String, and any type implementing `FromStr` (e.g.
//! i32, f64).
//!
//! The codegen is invoked via a generated build rule (e.g. `genrule`) and
//! produces a Rust file that should be included in your test file.

use std::{env, fs, path::Path};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use syn::{visit::Visit, Attribute, FnArg, ImplItem, ItemImpl, Pat, Type};

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
            regex_matcher: Regex::new(r"Cuke:\s*(Given|When|Then|Before|After)(?:\s+(.*))?")
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
                            // DataTable is implied by step context, not regex
                            // capture. So we do NOT
                            // consume an arg_index from `args` (captures).
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

        // If it's a method call: w.method(...)
        // If it's a function call: method(w, ...)

        let call_stmt = if struct_type.is_some() {
            quote! { w.#method_name(#(#method_call_args),*).await; }
        } else {
            quote! { #method_name(#(#method_call_args),*).await; }
        };

        // `cuke.rs` defines `AsyncStep` taking `StepContext`. Use `StepContext` in
        // wrapper signature.

        let glue = quote! {
            fn #wrapper_name(w: &mut #world_type, #[allow(unused_variables)] args: Vec<String>, #[allow(unused_variables)] ctx: netsim_testing::cuke::StepContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
                Box::pin(async move {
                    #(#args_parsing)*
                    #call_stmt
                })
            }
        };

        self.glue_code.push(glue);

        let keyword_ident = format_ident!("{}", keyword.to_lowercase());

        // Hooks (Before/After) don't take a pattern in registration.

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
    if attr.path().is_ident("doc") {
        if let syn::Meta::NameValue(meta) = &attr.meta {
            if let syn::Expr::Lit(expr_lit) = &meta.value {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    return Some(lit_str.value());
                }
            }
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cuke_codegen <input_file> <output_file>");
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    let content = fs::read_to_string(input_path).expect("Failed to read input file");
    let syntax = syn::parse_file(&content).expect("Failed to parse Rust code");

    let mut visitor = StepVisitor::new();
    visitor.visit_file(&syntax);

    let glue_code = &visitor.glue_code;
    let registration_code = &visitor.registration_code;

    // Default to 'W' generic if no struct found (though this likely won't compile
    // without where clause) But if no steps found, empty output is fine.

    let register_fn = if let Some(struct_type) = &visitor.struct_type {
        quote! {
            pub fn register_steps(c: &mut Cuke<#struct_type>) {
                #(#registration_code)*
            }
        }
    } else {
        // No steps found, generate generic empty register to avoid build errors?
        quote! {
            pub fn register_steps<W>(c: &mut Cuke<W>) {}
        }
    };

    let generated_rs = quote! {
        // Generated by cuke_codegen
        // Generated by cuke_codegen

        #(#glue_code)*

        #register_fn
    };

    // Use rustfmt-friendly output by converting to string
    let output_str = generated_rs.to_string();

    fs::write(output_path, output_str).expect("Failed to write output file");
}
