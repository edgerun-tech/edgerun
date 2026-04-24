//! Zero-dependency Error derive macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::*;

#[proc_macro_derive(Error)]
pub fn error_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
    let arms = match &input.data {
        Data::Enum(data) => {
            data.variants.iter().map(|v| {
                let ident = &v.ident;
                match &v.fields {
                    Fields::Unit => quote! { Self::#ident => write!(f, stringify!(#ident)), },
                    Fields::Unnamed(fields) => {
                        let n = fields.unnamed.len();
                        let pat = (0..n).map(|i| quote! { _ }).collect::<Vec<_>>();
                        quote! { Self::#ident(#(#pat),*) => write!(f, stringify!(#ident)), }
                    }
                    Fields::Named(_) => quote! { Self::#ident{..} => write!(f, stringify!(#ident)), },
                }
            }).collect::<Vec<_>>()
        }
        _ => panic!("Error derive only works on enums"),
    };
    
    quote! {
        impl #impl_generics std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self { #(#arms)* }
            }
        }
        
        impl #impl_generics std::error::Error for #name #ty_generics #where_clause {}
    }.into()
}