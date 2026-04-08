// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::str;

use nom::IResult;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

#[allow(dead_code)]
trait Parsable<'a>: Sized {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self>;
}

impl Parsable<'_> for u8 {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::combinator::map_res(
            nom::combinator::map_res(nom::character::complete::digit1, str::from_utf8),
            |s: &str| s.parse::<u8>(),
        )(input)
    }
}

impl Parsable<'_> for u16 {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::combinator::map_res(
            nom::combinator::map_res(nom::character::complete::digit1, str::from_utf8),
            |s: &str| s.parse::<u16>(),
        )(input)
    }
}

impl Parsable<'_> for u32 {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::combinator::map_res(
            nom::combinator::map_res(nom::character::complete::digit1, str::from_utf8),
            |s: &str| s.parse::<u32>(),
        )(input)
    }
}

#[proc_macro_derive(CommandParser, attributes(command, parser))]
pub fn command_parser_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let has_lifetime = !input.generics.lifetimes().collect::<Vec<_>>().is_empty();

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => panic!("#[derive(CommandParser)] is only supported for enums"),
    };

    let mut parser_fns = Vec::new();
    for variant in variants {
        let variant_name = &variant.ident;
        let parser_fn_name = Ident::new(
            &format!("parse_{}", variant_name.to_string().to_lowercase()),
            variant_name.span(),
        );

        let tag_attr = variant
            .attrs
            .iter()
            .find(|a| a.path().is_ident("command"))
            .expect("All variants must have a #[command(tag = ...)] attribute");
        let tag = if let syn::Meta::List(list) = &tag_attr.meta {
            let nested =
                list.parse_args::<syn::MetaNameValue>().expect("Failed to parse nested meta");
            if let syn::Expr::Lit(expr_lit) = nested.value {
                if let syn::Lit::Str(lit_str) = expr_lit.lit {
                    let b = syn::LitByteStr::new(lit_str.value().as_bytes(), lit_str.span());
                    quote! { #b }
                } else {
                    panic!("command attribute value must be a string literal");
                }
            } else {
                panic!("command attribute value must be a literal expression");
            }
        } else {
            panic!("command attribute must be a list");
        };

        let (field_names, field_parsers) = match &variant.fields {
            Fields::Unnamed(fields) => {
                let mut names = Vec::new();
                let mut parsers = Vec::new();
                for (i, field) in fields.unnamed.iter().enumerate() {
                    let field_name = Ident::new(&format!("field_{}", i), variant_name.span());
                    names.push(quote! { #field_name });
                    let field_type = &field.ty;

                    if let Some(parser_attr) =
                        field.attrs.iter().find(|a| a.path().is_ident("parser"))
                    {
                        let parser: syn::Expr = parser_attr.parse_args().unwrap();
                        parsers.push(quote! { let (input, #field_name) = #parser(input)?; });
                    } else {
                        let parser = get_parser_for_type(field_type);
                        parsers.push(quote! { let (input, #field_name) = #parser(input)?; });
                    }
                    parsers.push(quote! { let (input, _) = nom::combinator::opt(nom::bytes::complete::tag(b","))(input)?; });
                }
                (quote! { ( #(#names),* ) }, parsers)
            }
            Fields::Named(fields) => {
                let mut names = Vec::new();
                let mut parsers = Vec::new();
                for field in &fields.named {
                    let field_name = field.ident.as_ref().unwrap();
                    names.push(quote! { #field_name });
                    let field_type = &field.ty;

                    if let Some(parser_attr) =
                        field.attrs.iter().find(|a| a.path().is_ident("parser"))
                    {
                        let parser: syn::Expr = parser_attr.parse_args().unwrap();
                        parsers.push(quote! { let (input, #field_name) = #parser(input)?; });
                    } else {
                        let parser = get_parser_for_type(field_type);
                        parsers.push(quote! { let (input, #field_name) = #parser(input)?; });
                    }
                    parsers.push(quote! { let (input, _) = nom::combinator::opt(nom::bytes::complete::tag(b","))(input)?; });
                }
                (quote! { { #(#names),* } }, parsers)
            }
            Fields::Unit => (quote! {}, vec![]),
        };

        let parser_fn = if has_lifetime {
            quote! {
                fn #parser_fn_name(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
                    let (input, _) = nom::bytes::complete::tag(#tag)(input)?;
                    #(#field_parsers)*
                    Ok((input, #name::#variant_name #field_names))
                }
            }
        } else {
            quote! {
                fn #parser_fn_name(input: &[u8]) -> nom::IResult<&[u8], Self> {
                    let (input, _) = nom::bytes::complete::tag(#tag)(input)?;
                    #(#field_parsers)*
                    Ok((input, #name::#variant_name #field_names))
                }
            }
        };
        parser_fns.push(parser_fn);
    }

    let parser_fn_names = variants.iter().map(|v| {
        let variant_name = &v.ident;
        Ident::new(
            &format!("parse_{}", variant_name.to_string().to_lowercase()),
            variant_name.span(),
        )
    });

    let impl_block = if has_lifetime {
        quote! {
            impl<'a> #name<'a> {
                #(#parser_fns)*

                pub fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
                    #(
                        if let Ok(result) = Self::#parser_fn_names(input) {
                            return Ok(result);
                        }
                    )*
                    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Alt)))
                }
            }
        }
    } else {
        quote! {
            impl #name {
                #(#parser_fns)*

                pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
                    #(
                        if let Ok(result) = Self::#parser_fn_names(input) {
                            return Ok(result);
                        }
                    )*
                    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Alt)))
                }
            }
        }
    };

    TokenStream::from(impl_block)
}

fn get_parser_for_type(ty: &Type) -> proc_macro2::TokenStream {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments[0].arguments
            {
                if args.args.len() == 1 {
                    if let syn::GenericArgument::Type(inner_ty) = &args.args[0] {
                        let inner_parser = get_parser_for_type(inner_ty);
                        return quote! { nom::combinator::opt(#inner_parser) };
                    }
                }
            }
        }
    }

    quote! { <#ty as Parsable>::parse }
}
