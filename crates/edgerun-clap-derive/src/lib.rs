use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Variant, Field, Attribute, Meta, MetaList, Lit};

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

#[allow(clippy::type_complexity)]
fn parse_arg_attrs(field: &Field) -> (Option<char>, Option<String>, Option<String>, Option<String>, bool, Option<usize>, Option<char>) {
    let mut short = None;
    let mut long = None;
    let mut help = None;
    let mut default = None;
    let mut is_subcommand = false;
    let mut num_args = None;
    let mut value_delimiter = None;

    for attr in &field.attrs {
        if attr.path().is_ident("arg") {
            if let Meta::List(list) = &attr.meta {
                let mut tokens = list.tokens.clone().into_iter().peekable();
                while let Some(token) = tokens.next() {
                    if let proc_macro2::TokenTree::Ident(ident) = token {
                        let ident_str = ident.to_string();
                        if ident_str == "subcommand" {
                            is_subcommand = true;
                        } else if ident_str == "short" || ident_str == "long" || ident_str == "help" || ident_str == "default_value" || ident_str == "num_args" || ident_str == "value_delimiter" {
                            let mut value_tokens = proc_macro2::TokenStream::new();
                            while let Some(t) = tokens.peek() {
                                if let proc_macro2::TokenTree::Punct(p) = t {
                                    if p.as_char() == '=' {
                                        tokens.next();
                                        break;
                                    }
                                }
                                value_tokens.extend([tokens.next().unwrap()]);
                            }
                            if ident_str == "short" {
                                if let Ok(Lit::Char(c)) = syn::parse2::<Lit>(value_tokens) {
                                    short = Some(c.value());
                                }
                            } else if ident_str == "long" {
                                if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(value_tokens) {
                                    long = Some(s.value());
                                }
                            } else if ident_str == "help" {
                                if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(value_tokens) {
                                    help = Some(s.value());
                                }
                            } else if ident_str == "default_value" {
                                if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(value_tokens) {
                                    default = Some(s.value());
                                }
                            } else if ident_str == "num_args" {
                                if let Ok(Lit::Int(i)) = syn::parse2::<Lit>(value_tokens) {
                                    num_args = Some(i.base10_parse().unwrap_or(1));
                                }
                            } else if ident_str == "value_delimiter" {
                                if let Ok(Lit::Char(c)) = syn::parse2::<Lit>(value_tokens) {
                                    value_delimiter = Some(c.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
(short, long, help, default, is_subcommand, num_args, value_delimiter)
}

fn parse_global_command_attrs(attrs: &[Attribute]) -> (Option<String>, Option<String>) {
    let mut version = None;
    let mut author = None;

    for attr in attrs {
        if attr.path().is_ident("command") {
            if let Meta::List(list) = &attr.meta {
                let mut tokens = list.tokens.clone().into_iter().peekable();
                while let Some(token) = tokens.next() {
                    if let proc_macro2::TokenTree::Ident(ident) = token {
                        let ident_str = ident.to_string();
                        if ident_str == "version" || ident_str == "author" {
                            let mut value_tokens = proc_macro2::TokenStream::new();
                            while let Some(t) = tokens.peek() {
                                if let proc_macro2::TokenTree::Punct(p) = t {
                                    if p.as_char() == '=' {
                                        tokens.next();
                                        break;
                                    }
                                }
                                value_tokens.extend([tokens.next().unwrap()]);
                            }
                            if ident_str == "version" {
                                if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(value_tokens) {
                                    version = Some(s.value());
                                }
                            } else if ident_str == "author" {
                                if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(value_tokens) {
                                    author = Some(s.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (version, author)
}

fn parse_command_attrs(attrs: &[Attribute]) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut about = None;

    for attr in attrs {
        if attr.path().is_ident("command") {
            if let Meta::List(list) = &attr.meta {
                let mut tokens = list.tokens.clone().into_iter().peekable();
                while let Some(token) = tokens.next() {
                    if let proc_macro2::TokenTree::Ident(ident) = token {
                        let ident_str = ident.to_string();
                        if ident_str == "name" || ident_str == "about" {
                            let mut value_tokens = proc_macro2::TokenStream::new();
                            while let Some(t) = tokens.peek() {
                                if let proc_macro2::TokenTree::Punct(p) = t {
                                    if p.as_char() == '=' {
                                        tokens.next();
                                        break;
                                    }
                                }
                                value_tokens.extend([tokens.next().unwrap()]);
                            }
                            if ident_str == "name" {
                                if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(value_tokens) {
                                    name = Some(s.value());
                                }
                            } else if ident_str == "name" || ident_str == "about" {
                                if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(value_tokens) {
                                    about = Some(s.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (name, about)
}

#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn arg(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
            let variants: Vec<_> = e.variants.iter().collect();
            
            let variant_cmds: Vec<_> = variants.iter()
                .map(|v| {
                    let var_ident = &v.ident;
                    let var_name = v.ident.to_string();
                    let (name, about) = parse_command_attrs(&v.attrs);
                    let cmd_name = name.unwrap_or_else(|| to_kebab_case(&var_name));
                    let cmd_about = about;

                    let fields: Vec<_> = if let Fields::Named(named) = &v.fields {
                        named.named.iter().collect()
                    } else {
                        vec![]
                    };

                    let arg_builds: Vec<_> = fields.iter()
                        .map(|f| {
                            let field_name = f.ident.as_ref().unwrap().to_string();
                            let (short, long, help, default, is_subcommand, num_args, value_delimiter) = parse_arg_attrs(f);
                            let arg_long = long.unwrap_or_else(|| to_kebab_case(&field_name));
                            let mut arg = quote! { edgerun_clap::cli::Arg::new(#field_name).long(#arg_long) };
                            if is_subcommand {
                                arg = quote! { #arg.subcommand() };
                            }
                            if let Some(s) = short {
                                arg = quote! { #arg.short(#s) };
                            }
                            if let Some(h) = help {
                                arg = quote! { #arg.help(#h) };
                            }
                            if let Some(d) = default {
                                arg = quote! { #arg.default_value(#d) };
                            }
                            if let Some(n) = num_args {
                                arg = quote! { #arg.num_args(#n) };
                            }
                            if let Some(delim) = value_delimiter {
                                arg = quote! { #arg.value_delimiter(#delim) };
                            }
                            arg
                        })
                        .collect();

                    let from_body = if fields.is_empty() {
                        quote! { Self::#var_ident }
                    } else {
                        let field_loads: Vec<_> = fields.iter()
                            .map(|f| {
                                let ident = f.ident.as_ref().unwrap();
                                let field_name = ident.to_string();
                                quote! {
                                    #ident: matches.get_one::<String>(#field_name).cloned().unwrap_or_default()
                                }
                            })
                            .collect();
                        quote! { Self::#var_ident { #(#field_loads),* } }
                    };

                    (cmd_name, cmd_about, arg_builds, from_body, var_ident)
                })
                .collect();

            let subcommand_names: Vec<_> = variant_cmds.iter().map(|(n, _, _, _, _)| n.as_str()).collect();
            let subcommand_bodies: Vec<_> = variant_cmds.iter().map(|(_, _, _, body, _)| quote! { #body }).collect();
            let subcommand_idents: Vec<_> = variant_cmds.iter().map(|(_, _, _, _, i)| quote! { Self::#i }).collect();

            let subcommands_build: Vec<_> = variant_cmds.iter()
                .map(|(name, about, args, _, _)| {
                    let mut cmd = quote! { edgerun_clap::cli::Command::new(#name) };
                    if let Some(ab) = about {
                        cmd = quote! { #cmd.about(#ab) };
                    }
                    for arg in args {
                        cmd = quote! { #cmd.arg(#arg) };
                    }
                    cmd
                })
                .collect();

            quote! {
                impl #impl_generics edgerun_clap::Parser for #name #ty_generics #where_clause {
                    fn command() -> edgerun_clap::cli::Command {
                        let mut cmd = edgerun_clap::cli::Command::new(#cmd_name);
                        #(
                            cmd = cmd.subcommand(#subcommands_build);
                        )*
                        cmd
                    }

                    fn from(matches: &edgerun_clap::cli::ArgMatches) -> Self {
                        let sub = matches.positional.first().cloned().unwrap_or_default();
                        match sub.as_str() {
                            #(
                                #subcommand_names => #subcommand_idents,
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
                named.named.iter().collect()
            } else {
                vec![]
            };

            let (version, author) = parse_global_command_attrs(&input.attrs);
            let mut cmd = quote! { edgerun_clap::cli::Command::new(#cmd_name) };
            if let Some(v) = version {
                cmd = quote! { #cmd.version(#v) };
            }
            if let Some(a) = author {
                cmd = quote! { #cmd.author(#a) };
            }

            let arg_builds: Vec<_> = fields.iter()
                .map(|f| {
                    let field_name = f.ident.as_ref().unwrap().to_string();
                    let (short, long, help, default, is_subcommand, num_args, value_delimiter) = parse_arg_attrs(f);
                    let arg_long = long.unwrap_or_else(|| to_kebab_case(&field_name));
                    let mut arg = quote! { edgerun_clap::cli::Arg::new(#field_name).long(#arg_long) };
                    if is_subcommand {
                        arg = quote! { #arg.subcommand() };
                    }
                    if let Some(s) = short {
                        arg = quote! { #arg.short(#s) };
                    }
                    if let Some(h) = help {
                        arg = quote! { #arg.help(#h) };
                    }
                    if let Some(d) = default {
                        arg = quote! { #arg.default_value(#d) };
                    }
                    if let Some(n) = num_args {
                        arg = quote! { #arg.num_args(#n) };
                    }
                    if let Some(delim) = value_delimiter {
                        arg = quote! { #arg.value_delimiter(#delim) };
                    }
                    arg
                })
                .collect();

            let field_loads: Vec<_> = fields.iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let field_name = ident.to_string();
                    quote! {
                        #ident: matches.get_one::<String>(#field_name).cloned().unwrap_or_default()
                    }
                })
                .collect();

            quote! {
                impl #impl_generics edgerun_clap::Parser for #name #ty_generics #where_clause {
                    fn command() -> edgerun_clap::cli::Command {
                        let mut cmd = #cmd;
                        #(
                            cmd = cmd.arg(#arg_builds);
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

#[proc_macro_derive(Subcommand, attributes(command))]
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