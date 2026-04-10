//! AST-level Rust code editor — no more sed/heredocs/string surgery.
use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "edgerun-edit", about = "AST-level Rust code editor")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Dump { file: PathBuf },
    List { file: PathBuf },
    FindFn { file: PathBuf, name: String },
    ReplaceFnBody { file: PathBuf, name: String, new_body: String },
    AddFn { file: PathBuf, name: String, args: String, ret: String, body: String },
    RenameType { file: PathBuf, old: String, new: String },
    AddUse { file: PathBuf, use_path: String },
    AddDerive { file: PathBuf, name: String, derive: String },
    RemoveFn { file: PathBuf, name: String },
}

fn parse_file(path: &PathBuf) -> syn::File {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path.display(), e);
        std::process::exit(1);
    });
    syn::parse_file(&src).unwrap_or_else(|e| {
        eprintln!("Error parsing {}: {}", path.display(), e);
        std::process::exit(1);
    })
}

fn write_file(path: &PathBuf, file: &syn::File) {
    let formatted = prettyplease::unparse(file);
    std::fs::write(path, formatted).unwrap_or_else(|e| {
        eprintln!("Error writing {}: {}", path.display(), e);
        std::process::exit(1);
    });
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Dump { file } => {
            let f = parse_file(&file);
            println!("{:#?}", f);
        }
        Command::List { file } => {
            let f = parse_file(&file);
            for item in &f.items {
                match item {
                    syn::Item::Fn(func) => {
                        let args: Vec<String> = func.sig.inputs.iter()
                            .map(|a| quote::quote!(#a).to_string()).collect();
                        let ret = match &func.sig.output {
                            syn::ReturnType::Default => "()".to_string(),
                            syn::ReturnType::Type(_, ty) => quote::quote!(#ty).to_string(),
                        };
                        println!("fn {}({}) -> {}", func.sig.ident, args.join(", "), ret);
                    }
                    syn::Item::Struct(s) => println!("struct {}", s.ident),
                    syn::Item::Enum(e) => println!("enum {}", e.ident),
                    syn::Item::Impl(i) => println!("impl {}", quote::quote!(#i.self_ty)),
                    syn::Item::Mod(m) => println!("mod {}", m.ident),
                    syn::Item::Use(u) => println!("use {}", quote::quote!(#u)),
                    syn::Item::Const(c) => println!("const {}", c.ident),
                    syn::Item::Static(s) => println!("static {}", s.ident),
                    syn::Item::Trait(t) => println!("trait {}", t.ident),
                    syn::Item::Type(t) => println!("type {}", t.ident),
                    syn::Item::ExternCrate(e) => println!("extern crate {}", e.ident),
                    syn::Item::Union(u) => println!("union {}", u.ident),
                    _ => {}
                }
            }
        }
        Command::FindFn { file, name } => {
            let f = parse_file(&file);
            for item in &f.items {
                if let syn::Item::Fn(func) = item {
                    if func.sig.ident == name {
                        let out = syn::File { shebang: None, attrs: vec![], items: vec![item.clone()] };
                        println!("{}", prettyplease::unparse(&out));
                        return;
                    }
                }
            }
            eprintln!("fn {} not found", name);
        }
        Command::ReplaceFnBody { file, name, new_body } => {
            let mut f = parse_file(&file);
            let body: syn::Block = syn::parse_str(&format!("{{ {} }}", new_body))
                .unwrap_or_else(|e| { eprintln!("Invalid body: {}", e); std::process::exit(1) });
            for item in &mut f.items {
                if let syn::Item::Fn(func) = item {
                    if func.sig.ident == name {
                        func.block = Box::new(body);
                        write_file(&file, &f);
                        println!("Replaced body of fn {}", name);
                        return;
                    }
                }
            }
            eprintln!("fn {} not found", name);
        }
        Command::AddFn { file, name, args, ret, body } => {
            let mut f = parse_file(&file);
            // Parse args as a simple string, build FnArgs manually
            let mut inputs = syn::punctuated::Punctuated::new();
            if !args.trim().is_empty() {
                let parts: Vec<&str> = args.split(',').collect();
                for (i, part) in parts.iter().enumerate() {
                    let part = part.trim();
                    if part.is_empty() { continue; }
                    // Try to parse as fn arg
                    if let Ok(arg) = syn::parse_str::<syn::FnArg>(part) {
                        inputs.push(arg);
                        if i < parts.len() - 1 {
                            // comma added by punctuated
                        }
                    } else {
                        eprintln!("Invalid arg: {}", part);
                        std::process::exit(1);
                    }
                }
            }
            let ret_type: syn::ReturnType = if ret.is_empty() {
                syn::ReturnType::Default
            } else {
                let ty: syn::Type = syn::parse_str(&ret)
                    .unwrap_or_else(|e| { eprintln!("Invalid ret: {}", e); std::process::exit(1) });
                syn::ReturnType::Type(Default::default(), Box::new(ty))
            };
            let body_block: syn::Block = syn::parse_str(&body)
                .unwrap_or_else(|e| { eprintln!("Invalid body: {}", e); std::process::exit(1) });

            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            let new_fn = syn::ItemFn {
                attrs: vec![],
                vis: syn::Visibility::Public(syn::token::Pub::default()),
                sig: syn::Signature {
                    constness: None, asyncness: None, unsafety: None,
                    abi: None, fn_token: Default::default(), ident,
                    generics: Default::default(), paren_token: Default::default(),
                    inputs, variadic: None, output: ret_type,
                },
                block: Box::new(body_block),
            };
            f.items.push(syn::Item::Fn(new_fn));
            write_file(&file, &f);
            println!("Added fn {}", name);
        }
        Command::RenameType { file, old, new } => {
            let mut f = parse_file(&file);
            rename_type_in_file(&mut f, &old, &new);
            write_file(&file, &f);
            println!("Renamed {} -> {}", old, new);
        }
        Command::AddUse { file, use_path } => {
            let mut f = parse_file(&file);
            let use_item: syn::ItemUse = syn::parse_str(&format!("pub use {};", use_path))
                .unwrap_or_else(|e| { eprintln!("Invalid use: {}", e); std::process::exit(1) });
            f.items.insert(0, syn::Item::Use(use_item));
            write_file(&file, &f);
            println!("Added use {}", use_path);
        }
        Command::AddDerive { file, name, derive } => {
            let mut f = parse_file(&file);
            let derive_paths: Vec<syn::Path> = derive.split(',')
                .map(|d| syn::parse_str(d.trim())
                    .unwrap_or_else(|e| { eprintln!("Invalid derive: {}", e); std::process::exit(1) }))
                .collect();
            for item in &mut f.items {
                match item {
                    syn::Item::Struct(s) if s.ident == name => {
                        let existing = collect_derives(&s.attrs);
                        let all: syn::punctuated::Punctuated<syn::Path, syn::Token![,]> =
                            existing.into_iter().chain(derive_paths).collect();
                        s.attrs.retain(|a| !a.path().is_ident("derive"));
                        s.attrs.push(syn::parse_quote!(#[derive(#all)]));
                        write_file(&file, &f);
                        println!("Added #[derive({})] to {}", derive, name);
                        return;
                    }
                    syn::Item::Enum(e) if e.ident == name => {
                        let existing = collect_derives(&e.attrs);
                        let all: syn::punctuated::Punctuated<syn::Path, syn::Token![,]> =
                            existing.into_iter().chain(derive_paths).collect();
                        e.attrs.retain(|a| !a.path().is_ident("derive"));
                        e.attrs.push(syn::parse_quote!(#[derive(#all)]));
                        write_file(&file, &f);
                        println!("Added #[derive({})] to {}", derive, name);
                        return;
                    }
                    _ => {}
                }
            }
            eprintln!("struct/enum {} not found", name);
        }
        Command::RemoveFn { file, name } => {
            let mut f = parse_file(&file);
            let before = f.items.len();
            f.items.retain(|item| {
                if let syn::Item::Fn(func) = item { func.sig.ident != name } else { true }
            });
            if f.items.len() < before {
                write_file(&file, &f);
                println!("Removed fn {}", name);
            } else {
                eprintln!("fn {} not found", name);
            }
        }
    }
}

fn collect_derives(attrs: &[syn::Attribute]) -> Vec<syn::Path> {
    attrs.iter()
        .filter(|a| a.path().is_ident("derive"))
        .flat_map(|a| {
            a.parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                .map(|p| p.into_iter().collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .collect()
}

fn rename_type_in_file(file: &mut syn::File, old: &str, new: &str) {
    for item in &mut file.items { rename_type_in_item(item, old, new); }
}

fn rename_type_in_item(item: &mut syn::Item, old: &str, new: &str) {
    match item {
        syn::Item::Fn(func) => {
            for input in &mut func.sig.inputs {
                if let syn::FnArg::Typed(pat) = input { rename_type_in_type(&mut pat.ty, old, new); }
            }
            if let syn::ReturnType::Type(_, ty) = &mut func.sig.output { rename_type_in_type(ty, old, new); }
            rename_type_in_block(&mut func.block, old, new);
        }
        syn::Item::Struct(s) => { for field in s.fields.iter_mut() { rename_type_in_type(&mut field.ty, old, new); } }
        syn::Item::Enum(e) => {
            for variant in &mut e.variants {
                for field in &mut variant.fields { rename_type_in_type(&mut field.ty, old, new); }
            }
        }
        _ => {}
    }
}

fn rename_type_in_type(ty: &mut syn::Type, old: &str, new: &str) {
    match ty {
        syn::Type::Path(tp) => rename_type_in_type_path(tp, old, new),
        syn::Type::Reference(tr) => rename_type_in_type(&mut tr.elem, old, new),
        syn::Type::Slice(ts) => rename_type_in_type(&mut ts.elem, old, new),
        syn::Type::Array(ta) => rename_type_in_type(&mut ta.elem, old, new),
        syn::Type::Tuple(tt) => { for elem in &mut tt.elems { rename_type_in_type(elem, old, new); } }
        syn::Type::Group(tg) => rename_type_in_type(&mut tg.elem, old, new),
        syn::Type::Paren(tp) => rename_type_in_type(&mut tp.elem, old, new),
        syn::Type::BareFn(bf) => {
            for input in &mut bf.inputs { rename_type_in_type(&mut input.ty, old, new); }
            if let syn::ReturnType::Type(_, ret) = &mut bf.output { rename_type_in_type(ret, old, new); }
        }
        _ => {}
    }
}

fn rename_type_in_type_path(tp: &mut syn::TypePath, old: &str, new: &str) {
    if let Some(qself) = &mut tp.qself { rename_type_in_type(&mut qself.ty, old, new); }
    for seg in &mut tp.path.segments {
        if seg.ident == old { seg.ident = syn::Ident::new(new, seg.ident.span()); }
        if let syn::PathArguments::AngleBracketed(args) = &mut seg.arguments {
            for arg in &mut args.args {
                if let syn::GenericArgument::Type(ty) = arg { rename_type_in_type(ty, old, new); }
            }
        }
    }
}

fn rename_type_in_block(block: &mut syn::Block, old: &str, new: &str) {
    for stmt in &mut block.stmts {
        match stmt {
            syn::Stmt::Expr(e, _) => rename_type_in_expr(e, old, new),
            syn::Stmt::Local(local) => {
                if let Some(init) = &mut local.init { rename_type_in_expr(&mut init.expr, old, new); }
            }
            syn::Stmt::Item(item) => rename_type_in_item(item, old, new),
            _ => {}
        }
    }
}

fn rename_type_in_expr(expr: &mut syn::Expr, old: &str, new: &str) {
    match expr {
        syn::Expr::Path(ep) => {
            for seg in &mut ep.path.segments {
                if seg.ident == old { seg.ident = syn::Ident::new(new, seg.ident.span()); }
            }
        }
        syn::Expr::Call(ec) => {
            rename_type_in_expr(&mut ec.func, old, new);
            for arg in &mut ec.args { rename_type_in_expr(arg, old, new); }
        }
        syn::Expr::MethodCall(em) => {
            rename_type_in_expr(&mut em.receiver, old, new);
            for arg in &mut em.args { rename_type_in_expr(arg, old, new); }
        }
        syn::Expr::Block(eb) => rename_type_in_block(&mut eb.block, old, new),
        syn::Expr::Let(el) => rename_type_in_expr(&mut el.expr, old, new),
        syn::Expr::If(ei) => {
            rename_type_in_expr(&mut ei.cond, old, new);
            rename_type_in_block(&mut ei.then_branch, old, new);
            if let Some((_, else_branch)) = &mut ei.else_branch { rename_type_in_expr(else_branch, old, new); }
        }
        syn::Expr::Match(em) => {
            rename_type_in_expr(&mut em.expr, old, new);
            for arm in &mut em.arms {
                if let Some(guard) = &mut arm.guard { rename_type_in_expr(&mut guard.1, old, new); }
                rename_type_in_expr(&mut arm.body, old, new);
            }
        }
        syn::Expr::Closure(ec) => {
            for input in &mut ec.inputs {
                if let syn::Pat::Type(pt) = input { rename_type_in_type(&mut pt.ty, old, new); }
            }
            rename_type_in_expr(&mut ec.body, old, new);
        }
        syn::Expr::Assign(ea) => {
            rename_type_in_expr(&mut ea.left, old, new);
            rename_type_in_expr(&mut ea.right, old, new);
        }
        syn::Expr::Binary(eb) => {
            rename_type_in_expr(&mut eb.left, old, new);
            rename_type_in_expr(&mut eb.right, old, new);
        }
        syn::Expr::Unary(eu) => rename_type_in_expr(&mut eu.expr, old, new),
        syn::Expr::Return(er) => { if let Some(e) = &mut er.expr { rename_type_in_expr(e, old, new); } }
        syn::Expr::Loop(el) => rename_type_in_block(&mut el.body, old, new),
        syn::Expr::ForLoop(ef) => {
            rename_type_in_expr(&mut ef.expr, old, new);
            rename_type_in_block(&mut ef.body, old, new);
        }
        syn::Expr::While(ew) => {
            rename_type_in_expr(&mut ew.cond, old, new);
            rename_type_in_block(&mut ew.body, old, new);
        }
        syn::Expr::Struct(es) => {
            for field in &mut es.fields { rename_type_in_expr(&mut field.expr, old, new); }
        }
        syn::Expr::Array(ea) => { for elem in &mut ea.elems { rename_type_in_expr(elem, old, new); } }
        syn::Expr::Tuple(et) => { for elem in &mut et.elems { rename_type_in_expr(elem, old, new); } }
        syn::Expr::Paren(ep) => rename_type_in_expr(&mut ep.expr, old, new),
        syn::Expr::Index(ei) => {
            rename_type_in_expr(&mut ei.expr, old, new);
            rename_type_in_expr(&mut ei.index, old, new);
        }
        syn::Expr::Field(ef) => rename_type_in_expr(&mut ef.base, old, new),
        _ => {}
    }
}
