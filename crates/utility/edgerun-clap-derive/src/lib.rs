use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Field, Fields, Lit, Meta};

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

fn parse_arg_attrs(field: &Field) -> ArgAttrs {
    let mut attrs = ArgAttrs::default();
    for attr in &field.attrs {
        if !attr.path().is_ident("arg") {
            continue;
        }
        if let Meta::List(list) = &attr.meta {
            let mut tokens = list.tokens.clone().into_iter().peekable();
            while let Some(token) = tokens.next() {
                if let proc_macro2::TokenTree::Ident(ident) = token {
                    let ident_str = ident.to_string();
                    match ident_str.as_str() {
                        "subcommand" => attrs.is_subcommand = true,
                        "short" => {
                            let val = collect_value(&mut tokens);
                            if let Ok(Lit::Char(c)) = syn::parse2::<Lit>(val) {
                                attrs.short = Some(c.value());
                            }
                        }
                        "long" => {
                            let val = collect_value(&mut tokens);
                            if val.is_empty() {
                                attrs.long = Some(String::new());
                            } else if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(val) {
                                attrs.long = Some(s.value());
                            }
                        }
                        "help" => {
                            let val = collect_value(&mut tokens);
                            if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(val) {
                                attrs.help = Some(s.value());
                            }
                        }
                        "default_value" => {
                            let val = collect_value(&mut tokens);
                            if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(val) {
                                attrs.default_value = Some(s.value());
                            }
                        }
                        "num_args" => {
                            let val = collect_value(&mut tokens);
                            if let Ok(Lit::Int(i)) = syn::parse2::<Lit>(val) {
                                attrs.num_args = Some(i.base10_parse().unwrap_or(1));
                            }
                        }
                        "value_delimiter" => {
                            let val = collect_value(&mut tokens);
                            if let Ok(Lit::Char(c)) = syn::parse2::<Lit>(val) {
                                attrs.value_delimiter = Some(c.value());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    attrs
}

fn collect_value(
    tokens: &mut std::iter::Peekable<proc_macro2::token_stream::IntoIter>,
) -> proc_macro2::TokenStream {
    let mut val = proc_macro2::TokenStream::new();
    while let Some(t) = tokens.peek() {
        if let proc_macro2::TokenTree::Punct(p) = t {
            if p.as_char() == ',' {
                break;
            }
            if p.as_char() == '=' {
                tokens.next();
                break;
            }
        }
        val.extend([tokens.next().unwrap()]);
    }
    val
}

#[derive(Default)]
struct ArgAttrs {
    short: Option<char>,
    long: Option<String>,
    help: Option<String>,
    default_value: Option<String>,
    is_subcommand: bool,
    num_args: Option<usize>,
    value_delimiter: Option<char>,
}

fn parse_struct_command_attrs(attrs: &[Attribute]) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut about = None;
    for attr in attrs {
        if !attr.path().is_ident("command") {
            continue;
        }
        if let Meta::List(list) = &attr.meta {
            let mut tokens = list.tokens.clone().into_iter().peekable();
            while let Some(token) = tokens.next() {
                if let proc_macro2::TokenTree::Ident(ident) = token {
                    let ident_str = ident.to_string();
                    match ident_str.as_str() {
                        "name" => {
                            let val = collect_value(&mut tokens);
                            if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(val) {
                                name = Some(s.value());
                            }
                        }
                        "about" => {
                            let val = collect_value(&mut tokens);
                            if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(val) {
                                about = Some(s.value());
                            }
                        }
                        "version" | "author" => {
                            collect_value(&mut tokens);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    (name, about)
}

fn is_option_type(field: &Field) -> bool {
    if let syn::Type::Path(tp) = &field.ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

fn is_bool_type(field: &Field) -> bool {
    if let syn::Type::Path(tp) = &field.ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "bool" && seg.arguments.is_empty();
        }
    }
    false
}

fn extract_inner_option_type(field: &Field) -> Option<syn::Type> {
    if let syn::Type::Path(tp) = &field.ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}

fn build_arg_expr(field_name: &str, attrs: &ArgAttrs, is_bool: bool) -> proc_macro2::TokenStream {
    let arg_long = if attrs.long.as_deref() == Some("") {
        to_kebab_case(field_name)
    } else {
        attrs
            .long
            .clone()
            .unwrap_or_else(|| to_kebab_case(field_name))
    };
    let mut arg = quote! { edgerun_clap::cli::Arg::new(#field_name).long(#arg_long) };
    if is_bool {
        arg = quote! { #arg.action(edgerun_clap::cli::Action::StoreTrue) };
    }
    if attrs.is_subcommand {
        arg = quote! { #arg.subcommand() };
    }
    if let Some(s) = attrs.short {
        arg = quote! { #arg.short(#s) };
    }
    if let Some(h) = &attrs.help {
        arg = quote! { #arg.help(#h) };
    }
    if let Some(d) = &attrs.default_value {
        arg = quote! { #arg.default_value(#d) };
    }
    if let Some(n) = attrs.num_args {
        arg = quote! { #arg.num_args(#n) };
    }
    if let Some(delim) = attrs.value_delimiter {
        arg = quote! { #arg.value_delimiter(#delim) };
    }
    arg
}

fn build_field_load(field: &Field) -> proc_macro2::TokenStream {
    let ident = field.ident.as_ref().unwrap();
    let field_name = ident.to_string();

    if is_bool_type(field) {
        return quote! {
            #ident: matches.get_flag(#field_name)
        };
    }

    if is_option_type(field) {
        let inner_ty = extract_inner_option_type(field).unwrap_or_else(|| field.ty.clone());
        return quote! {
            #ident: matches.get_one::<#inner_ty>(#field_name)
        };
    }

    quote! {
        #ident: matches.get_one::<String>(#field_name).unwrap_or_default()
    }
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

    let gen = match &input.data {
        Data::Enum(e) => {
            let variants: Vec<_> = e.variants.iter().collect();
            let variant_cmds: Vec<_> = variants
                .iter()
                .map(|v| {
                    let var_ident = &v.ident;
                    let var_name = v.ident.to_string();
                    let (cmd_name, cmd_about) = parse_struct_command_attrs(&v.attrs);
                    let cmd_name = cmd_name.unwrap_or_else(|| to_kebab_case(&var_name));

                    let fields: Vec<_> = if let Fields::Named(named) = &v.fields {
                        named.named.iter().collect()
                    } else {
                        vec![]
                    };

                    let arg_builds: Vec<_> = fields
                        .iter()
                        .map(|f| {
                            let field_name = f.ident.as_ref().unwrap().to_string();
                            let attrs = parse_arg_attrs(f);
                            let is_bool = is_bool_type(f);
                            build_arg_expr(&field_name, &attrs, is_bool)
                        })
                        .collect();

                    let field_loads: Vec<_> = fields.iter().map(|f| build_field_load(f)).collect();

                    let from_body = if fields.is_empty() {
                        quote! { Self::#var_ident }
                    } else {
                        quote! { Self::#var_ident { #(#field_loads),* } }
                    };

                    (cmd_name, cmd_about, arg_builds, from_body, var_ident)
                })
                .collect();

            let subcommand_names: Vec<_> = variant_cmds
                .iter()
                .map(|(n, _, _, _, _)| n.as_str())
                .collect();
            let subcommand_idents: Vec<_> = variant_cmds
                .iter()
                .map(|(_, _, _, _, i)| quote! { Self::#i })
                .collect();

            let subcommands_build: Vec<_> = variant_cmds
                .iter()
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
                        let mut cmd = edgerun_clap::cli::Command::new(#name);
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
                        #name
                    }
                }
            }
        }
        Data::Struct(s) => {
            let fields: Vec<_> = if let Fields::Named(named) = &s.fields {
                named.named.iter().collect()
            } else {
                vec![]
            };

            let (cmd_name_override, about_override) = parse_struct_command_attrs(&input.attrs);
            let cmd_name = cmd_name_override.unwrap_or_else(|| to_kebab_case(&name.to_string()));

            let mut cmd = quote! { edgerun_clap::cli::Command::new(#cmd_name) };
            if let Some(ab) = about_override {
                cmd = quote! { #cmd.about(#ab) };
            }

            let mut positional_idx: usize = 0;
            let arg_builds: Vec<_> = fields
                .iter()
                .map(|f| {
                    let field_name = f.ident.as_ref().unwrap().to_string();
                    let attrs = parse_arg_attrs(f);
                    let is_bool = is_bool_type(f);

                    let is_positional = attrs.short.is_none()
                        && attrs.long.is_none()
                        && !is_bool
                        && !attrs.is_subcommand;

                    if is_positional {
                        positional_idx += 1;
                        let mut arg =
                            quote! { edgerun_clap::cli::Arg::new(#field_name).positional() };
                        if let Some(h) = &attrs.help {
                            arg = quote! { #arg.help(#h) };
                        }
                        if let Some(d) = &attrs.default_value {
                            arg = quote! { #arg.default_value(#d) };
                        }
                        arg
                    } else {
                        build_arg_expr(&field_name, &attrs, is_bool)
                    }
                })
                .collect();

            positional_idx = 0;
            let field_loads: Vec<_> = fields
                .iter()
                .map(|f| {
                    let attrs = parse_arg_attrs(f);
                    let is_bool = is_bool_type(f);
                    let is_positional = attrs.short.is_none()
                        && attrs.long.is_none()
                        && !is_bool
                        && !attrs.is_subcommand;

                    if is_positional {
                        let ident = f.ident.as_ref().unwrap();
                        let idx = positional_idx;
                        positional_idx += 1;
                        if is_option_type(f) {
                            let inner_ty = extract_inner_option_type(f).unwrap_or_else(|| f.ty.clone());
                            quote! {
                                #ident: matches.get_positional(#idx).map(|s| s.parse::<#inner_ty>().unwrap())
                            }
                        } else {
                            quote! {
                                #ident: matches.get_positional(#idx).map(String::from).unwrap_or_default()
                            }
                        }
                    } else {
                        build_field_load(f)
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
                            #(#field_loads),*
                        }
                    }
                }
            }
        }
        _ => {
            quote! {
                impl #impl_generics edgerun_clap::Parser for #name #ty_generics #where_clause {
                    fn command() -> edgerun_clap::cli::Command {
                        edgerun_clap::cli::Command::new(#name)
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
