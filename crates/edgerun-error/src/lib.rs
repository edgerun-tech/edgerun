//! Zero-dependency Error derive macro.

#![allow(unused)]

use proc_macro::TokenStream;
use quote::quote;
use syn::*;

/// Derives the `Error` trait implementation.
#[proc_macro_derive(Error, attributes(error))]
pub fn error_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
    let variants = match &input.data {
        Data::Enum(e) => &e.variants,
        _ => panic!("Error derive only works on enums"),
    };
    
    let mut cases = Vec::new();
    
    for variant in variants.iter() {
        let variant_ident = &variant.ident;
        
        match &variant.fields {
            Fields::Unit => {
                cases.push(quote! { Self::#variant_ident => write!(f, "{}", stringify!(#variant_ident)) });
            }
            Fields::Unnamed(u) if u.unnamed.len() == 1 => {
                cases.push(quote! { Self::#variant_ident(ref e) => write!(f, "{}: {}", stringify!(#variant_ident), e) });
            }
            _ => {}
        }
    }
    
    quote! {
        impl #impl_generics ::core::fmt::Display for #ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(#cases),*
                }
            }
        }
        
        impl #impl_generics ::core::error::Error for #ident #ty_generics #where_clause {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                None
            }
        }
    }.into()
}