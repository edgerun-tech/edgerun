//! Zero-dependency Error derive macro.
//!
//! Supports:
//! - Unit variants: `#[error("message")]`
//! - Variants with a single field: `#[error("message: {0}")]`

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
            .map(|(Eq, d)| quote!(#d))
            .unwrap_or_else(|| quote!());

        let mut format_string = None;

        for attr in &variant.attrs {
            if attr.path().is_ident("error") {
                if let Meta::NameValue(meta) = &attr.meta {
                    if let Expr::Lit(expr) = &meta.value {
                        if let Lit::Str(lit_str) = &expr.lit {
                            format_string = Some(lit_str.value());
                        }
                    }
                }
            }
        }

        let format_str = format_string.unwrap_or_else(|| variant_ident.to_string());

        match &variant.fields {
            Fields::Unit => {
                match_arms.push(quote! {
                    #ident::#variant_ident #discriminant => write!(f, #format_str),
                });
            }
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                match_arms.push(quote! {
                    #ident::#variant_ident(ref __e) => write!(f, #format_str, __e),
                });
            }
            _ => {
                panic!("Error derive: variant must have 0 or 1 fields");
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
