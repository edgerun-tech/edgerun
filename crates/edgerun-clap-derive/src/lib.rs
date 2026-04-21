use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Variant};

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_derive(Parser, attributes(arg, command))]
pub fn derive_parser(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let cmd_name = to_kebab_case(&name.to_string());

    let gen = match &input.data {
        Data::Enum(e) => {
            let variants: Vec<_> = e.variants.iter()
                .map(|v| &v.ident)
                .collect();
            
            let variant_names: Vec<_> = variants.iter()
                .map(|v| v.to_string())
                .collect();

            let variant_idents: Vec<_> = variants.iter()
                .map(|v| quote! { Self::#v })
                .collect();

            quote! {
                impl #impl_generics edgerun_clap::Parser for #name #ty_generics #where_clause {
                    fn command() -> edgerun_clap::cli::Command {
                        edgerun_clap::cli::Command::new(#cmd_name)
                    }

                    fn from(matches: &edgerun_clap::cli::ArgMatches) -> Self {
                        let sub = matches.positional.first().cloned().unwrap_or_default();
                        match sub.as_str() {
                            #(
                                #variant_names => #variant_idents,
                            )*
                            _ => panic!("unknown subcommand: {}", sub),
                        }
                    }
                }

                impl #impl_generics edgerun_clap::cli::Subcommand for #name #ty_generics #where_clause {
                    fn name() -> &'static str {
                        #cmd_name
                    }
                }
            }
        },
        Data::Struct(s) => {
            let fields: Vec<_> = if let Fields::Named(named) = &s.fields {
                named.named.iter().filter_map(|f| f.ident.as_ref()).collect()
            } else {
                vec![]
            };

            let arg_names: Vec<_> = fields.iter().map(|f| f.to_string()).collect();

            let field_loads: Vec<_> = fields.iter()
                .map(|f| {
                    let ident = f.to_string();
                    quote! {
                        #f: matches.get_one::<String>(#ident).cloned().unwrap_or_default()
                    }
                })
                .collect();

            quote! {
                impl #impl_generics edgerun_clap::Parser for #name #ty_generics #where_clause {
                    fn command() -> edgerun_clap::cli::Command {
                        let mut cmd = edgerun_clap::cli::Command::new(#cmd_name);
                        #(
                            cmd = cmd.arg(edgerun_clap::cli::Arg::new(#arg_names));
                        )*
                        cmd
                    }

                    fn from(matches: &edgerun_clap::cli::ArgMatches) -> Self {
                        Self {
                            #(
                                #field_loads,
                            )*
                        }
                    }
                }
            }
        },
        _ => {
            quote! {
                impl #impl_generics edgerun_clap::Parser for #name #ty_generics #where_clause {
                    fn command() -> edgerun_clap::cli::Command {
                        edgerun_clap::cli::Command::new(#cmd_name)
                    }

                    fn from(_matches: &edgerun_clap::cli::ArgMatches) -> Self {
                        Self
                    }
                }
            }
        }
    };

    gen.into()
}

#[proc_macro_derive(Subcommand)]
pub fn derive_subcommand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let cmd_name = to_kebab_case(&name.to_string());

    let gen = quote! {
        impl #impl_generics edgerun_clap::cli::Subcommand for #name #ty_generics #where_clause {
            fn name() -> &'static str {
                #cmd_name
            }
        }
    };

    gen.into()
}