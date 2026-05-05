//! Zero-dependency Error derive macro.
//!
//! Supports:
//! - Unit variants: `#[error("message")]`
//! - Variants with unnamed fields: `#[error("message: {0}")]`

use proc_macro::TokenStream;
use quote::quote;
use syn::*;

#[proc_macro_derive(Error, attributes(error))]
pub fn error_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    let variants = match &input.data {
        Data::Enum(e) => &e.variants,
        _ => panic!("Error derive only works on enums"),
    };

    let mut match_arms = Vec::new();

    for variant in variants.iter() {
        let variant_ident = &variant.ident;
        let discriminant = variant
            .discriminant
            .as_ref()
            .map(|(_, d)| quote!(#d))
            .unwrap_or_else(|| quote!());

        let mut format_string = None;

        for attr in &variant.attrs {
            if !attr.path().is_ident("error") {
                continue;
            }

            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                format_string = Some(lit_str.value());
                continue;
            }

            if let Meta::NameValue(meta) = &attr.meta {
                if let Expr::Lit(expr) = &meta.value {
                    if let Lit::Str(lit_str) = &expr.lit {
                        format_string = Some(lit_str.value());
                    }
                }
            }
        }

        match &variant.fields {
            Fields::Unit => {
                let format_str = format_string.unwrap_or_else(|| variant_ident.to_string());
                match_arms.push(quote! {
                    #ident::#variant_ident #discriminant => write!(f, #format_str),
                });
            }
            Fields::Unnamed(unnamed) => {
                let bindings: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|idx| Ident::new(&format!("__field{idx}"), variant_ident.span()))
                    .collect();
                if let Some(format_str) = format_string {
                    let args_len = positional_arg_count(&format_str).unwrap_or(bindings.len());
                    let args = bindings.iter().take(args_len);
                    match_arms.push(quote! {
                        #ident::#variant_ident(#(ref #bindings),*) => write!(f, #format_str, #(#args),*),
                    });
                } else {
                    match_arms.push(quote! {
                        #ident::#variant_ident(#(ref #bindings),*) => {
                            write!(f, stringify!(#variant_ident))
                        }
                    });
                }
            }
            _ => {
                panic!("Error derive: variant fields must be unit or unnamed");
            }
        }
    }

    let expanded = quote! {
        impl #impl_generics ::core::fmt::Display for #ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(#match_arms)*
                }
            }
        }

        impl #impl_generics ::core::error::Error for #ident #ty_generics #where_clause {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                None
            }
        }
    };

    expanded.into()
}

fn positional_arg_count(format_str: &str) -> Option<usize> {
    let mut max_index: Option<usize> = None;
    let bytes = format_str.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        if bytes[idx] != b'{' || idx + 1 >= bytes.len() || bytes[idx + 1] == b'{' {
            idx += 1;
            continue;
        }

        let mut cursor = idx + 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }

        if cursor > start {
            if let Ok(position) = format_str[start..cursor].parse::<usize>() {
                max_index = Some(max_index.map_or(position, |max| max.max(position)));
            }
        } else if cursor < bytes.len() && (bytes[cursor] == b'}' || bytes[cursor] == b':') {
            return None;
        }

        idx = cursor + 1;
    }

    max_index.map(|index| index + 1)
}
