// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use syn::{Attribute, Type};

pub fn parse_doc_comment(attr: &Attribute) -> Option<String> {
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

/// Checks if an argument is of a specific type (e.g., "String" or "DataTable").
/// Returns true if the last segment of the path matches the type name.
pub fn is_arg_type(ty: &Type, type_name: &str) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == type_name;
        }
    }
    false
}
