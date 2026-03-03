// Copyright 2026 The Android Open Source Project

use std::{fs, path::Path};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use syn::{visit::Visit, Attribute, FnArg, ImplItem, ItemImpl, Pat, Type};

use crate::utils::{is_arg_type, parse_doc_comment};

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
        let mut steps: Vec<String> = Vec::new();
        let mut current_step = String::new();
        for attr in attrs {
            if let Some(c) = parse_doc_comment(attr) {
                let trimmed = c.trim_start();
                if trimmed.starts_with("STEP:") {
                    if !current_step.is_empty() {
                        steps.push(current_step);
                    }
                    current_step = trimmed.to_string();
                } else if !current_step.is_empty() {
                    current_step.push_str(" ");
                    current_step.push_str(c.trim());
                }
            }
        }
        if !current_step.is_empty() {
            steps.push(current_step);
        }

        for step_comment in steps {
            let caps = match self.regex_matcher.captures(step_comment.trim()) {
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
                        let is_string = is_arg_type(ty, "String");
                        let is_data_table = is_arg_type(ty, "DataTable");

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

pub fn process_rust_file(input_path: &Path, output_path: &Path) {
    let content = fs::read_to_string(input_path).expect("Failed to read input file");
    let syntax = syn::parse_file(&content).expect("Failed to parse Rust code");

    let mut visitor = StepVisitor::new();
    visitor.visit_file(&syntax);

    let glue_code = &visitor.glue_code;
    let mut registration_code = visitor.registration_code.clone();

    // Sort registration code by pattern length descending to ensure
    // specific patterns match before generic catch-all patterns.
    registration_code.sort_by(|a, b| {
        // We need to parse the Literal string length out of the TokenStream.
        // A simple heuristic is just sorting the TokenStream string representation
        // length descending. Since the pattern is the largest variable part of
        // the registration TokenStream, longer patterns will produce longer
        // total string lengths.
        let len_a = a.to_string().len();
        let len_b = b.to_string().len();
        len_b.cmp(&len_a)
    });

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
