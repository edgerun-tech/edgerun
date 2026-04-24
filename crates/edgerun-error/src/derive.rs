//! Error derive macro implementation.

use proc_macro::TokenStream;
use quote::quote;
use syn::*;

/// Derives the `Error` trait implementation.
///
/// Supports:
/// - Variants with a single `String` field: `#[error("message: {0}")]`
/// - Unit variants: `#[error("message")]`
///
/// Does NOT support:
/// - Multiple fields
/// - `#[source]` attribute
/// - `#[from]` attribute
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
    let mut static_strings = Vec::new();
    
    for variant in variants.iter() {
        let variant_ident = &variant.ident;
        let discriminant = variant.discriminant.as_ref()
            .map(|(Eq, d)| quote!(#d))
            .unwrap_or_else(|| quote!());
        
        let mut format_string = None;
        
        for attr in &variant.attrs {
            if attr.path().is_ident("error") {
                if let Ok(Meta::NameValue(meta)) = attr.parse_meta() {
                    if let Lit::Str(lit) = meta.lit {
                        format_string = Some(lit.value());
                    }
                }
            }
        }
        
        let format_str = format_string.unwrap_or_else(|| {
            // Default to variant name
            variant_ident.to_string()
        });
        
        static_strings.push(format_str.clone());
        
        match variant.fields {
            Fields::Unit => {
                match_arms.push(quote! {
                    #ident::#variant_ident #discriminant => write!(f, #format_str),
                });
            }
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                match_arms.push(quote! {
                    #ident::#variant_ident #discriminant(ref __e) => write!(f, #format_str, __e),
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