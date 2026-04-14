//! AST-level edit operations. All operations parse → transform → prettyplease → write.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use prettyplease::unparse as pretty_unparse;
use quote::ToTokens;
use syn::punctuated::Punctuated;

// ── File I/O ─────────────────────────────────────────────────────────────────

pub fn parse_file(path: &Path) -> Result<syn::File, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    syn::parse_file(&src).map_err(|e| format!("parsing {}: {e}", path.display()))
}

pub fn write_file(path: &Path, file: &syn::File) -> Result<(), String> {
    let formatted = pretty_unparse(file);
    std::fs::write(path, formatted).map_err(|e| format!("writing {}: {e}", path.display()))
}

// ── Rename type across all files ─────────────────────────────────────────────

pub fn rename_type_in_file(file: &mut syn::File, old: &str, new: &str) {
    for item in &mut file.items {
        rename_type_in_item(item, old, new);
    }
}

fn rename_type_in_item(item: &mut syn::Item, old: &str, new: &str) {
    match item {
        syn::Item::Use(u) => rename_type_in_use_tree(&mut u.tree, old, new),
        syn::Item::Struct(s) => {
            if s.ident == old {
                s.ident = syn::Ident::new(new, s.ident.span());
            }
            for field in s.fields.iter_mut() {
                rename_type_in_type(&mut field.ty, old, new);
            }
        }
        syn::Item::Enum(e) => {
            if e.ident == old {
                e.ident = syn::Ident::new(new, e.ident.span());
            }
            for variant in &mut e.variants {
                for field in &mut variant.fields {
                    rename_type_in_type(&mut field.ty, old, new);
                }
            }
        }
        syn::Item::Union(u) => {
            if u.ident == old {
                u.ident = syn::Ident::new(new, u.ident.span());
            }
            for field in u.fields.named.iter_mut() {
                rename_type_in_type(&mut field.ty, old, new);
            }
        }
        syn::Item::Type(t) => {
            if t.ident == old {
                t.ident = syn::Ident::new(new, t.ident.span());
            }
            rename_type_in_type(&mut t.ty, old, new);
        }
        syn::Item::Const(c) => {
            rename_type_in_type(&mut c.ty, old, new);
            rename_type_in_expr(&mut c.expr, old, new);
        }
        syn::Item::Static(s) => {
            rename_type_in_type(&mut s.ty, old, new);
            rename_type_in_expr(&mut s.expr, old, new);
        }
        syn::Item::Fn(func) => {
            for input in &mut func.sig.inputs {
                if let syn::FnArg::Typed(pat) = input {
                    rename_type_in_type(&mut pat.ty, old, new);
                }
            }
            if let syn::ReturnType::Type(_, ty) = &mut func.sig.output {
                rename_type_in_type(ty, old, new);
            }
            rename_type_in_block(&mut func.block, old, new);
        }
        syn::Item::Impl(imp) => {
            rename_type_in_type(&mut imp.self_ty, old, new);
            if let Some((_, trait_path, _)) = &mut imp.trait_ {
                rename_type_path(trait_path, old, new);
            }
            for item in &mut imp.items {
                if let syn::ImplItem::Fn(method) = item {
                    for input in &mut method.sig.inputs {
                        if let syn::FnArg::Typed(pat) = input {
                            rename_type_in_type(&mut pat.ty, old, new);
                        }
                    }
                    if let syn::ReturnType::Type(_, ty) = &mut method.sig.output {
                        rename_type_in_type(ty, old, new);
                    }
                    rename_type_in_block(&mut method.block, old, new);
                }
            }
        }
        syn::Item::Trait(tr) => {
            for item in &mut tr.items {
                if let syn::TraitItem::Fn(method) = item {
                    for input in &mut method.sig.inputs {
                        if let syn::FnArg::Typed(pat) = input {
                            rename_type_in_type(&mut pat.ty, old, new);
                        }
                    }
                    if let syn::ReturnType::Type(_, ty) = &mut method.sig.output {
                        rename_type_in_type(ty, old, new);
                    }
                    if let Some(default) = &mut method.default {
                        rename_type_in_block(default, old, new);
                    }
                }
            }
        }
        _ => {}
    }
}

fn rename_type_in_use_tree(tree: &mut syn::UseTree, old: &str, new: &str) {
    match tree {
        syn::UseTree::Path(up) => {
            if up.ident == old {
                up.ident = syn::Ident::new(new, up.ident.span());
            }
            rename_type_in_use_tree(&mut up.tree, old, new);
        }
        syn::UseTree::Name(un) => {
            if un.ident == old {
                un.ident = syn::Ident::new(new, un.ident.span());
            }
        }
        syn::UseTree::Rename(urn) => {
            if urn.rename == old {
                urn.rename = syn::Ident::new(new, urn.rename.span());
            }
            if urn.ident == old {
                urn.ident = syn::Ident::new(new, urn.ident.span());
            }
        }
        syn::UseTree::Group(ug) => {
            for child in &mut ug.items {
                rename_type_in_use_tree(child, old, new);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn rename_type_in_type(ty: &mut syn::Type, old: &str, new: &str) {
    match ty {
        syn::Type::Path(tp) => rename_type_path(&mut tp.path, old, new),
        syn::Type::Reference(tr) => rename_type_in_type(&mut tr.elem, old, new),
        syn::Type::Slice(ts) => rename_type_in_type(&mut ts.elem, old, new),
        syn::Type::Array(ta) => rename_type_in_type(&mut ta.elem, old, new),
        syn::Type::Tuple(tt) => {
            for elem in &mut tt.elems {
                rename_type_in_type(elem, old, new);
            }
        }
        syn::Type::Group(tg) => rename_type_in_type(&mut tg.elem, old, new),
        syn::Type::Paren(tp) => rename_type_in_type(&mut tp.elem, old, new),
        syn::Type::BareFn(bf) => {
            for input in &mut bf.inputs {
                rename_type_in_type(&mut input.ty, old, new);
            }
            if let syn::ReturnType::Type(_, ret) = &mut bf.output {
                rename_type_in_type(ret, old, new);
            }
        }
        _ => {}
    }
}

fn rename_type_path(path: &mut syn::Path, old: &str, new: &str) {
    for seg in &mut path.segments {
        if seg.ident == old {
            seg.ident = syn::Ident::new(new, seg.ident.span());
        }
        if let syn::PathArguments::AngleBracketed(args) = &mut seg.arguments {
            for arg in &mut args.args {
                if let syn::GenericArgument::Type(ty) = arg {
                    rename_type_in_type(ty, old, new);
                }
            }
        }
    }
}

fn rename_type_in_block(block: &mut syn::Block, old: &str, new: &str) {
    for stmt in &mut block.stmts {
        match stmt {
            syn::Stmt::Expr(e, _) => rename_type_in_expr(e, old, new),
            syn::Stmt::Local(local) => {
                rename_type_in_pat(&mut local.pat, old, new);
                if let Some(init) = &mut local.init {
                    rename_type_in_expr(&mut init.expr, old, new);
                }
            }
            syn::Stmt::Item(item) => rename_type_in_item(item, old, new),
            _ => {}
        }
    }
}

fn rename_type_in_pat(pat: &mut syn::Pat, old: &str, new: &str) {
    match pat {
        syn::Pat::Type(pt) => {
            rename_type_in_type(&mut pt.ty, old, new);
            rename_type_in_pat(&mut pt.pat, old, new);
        }
        syn::Pat::Struct(ps) => {
            rename_type_path(&mut ps.path, old, new);
            for field in &mut ps.fields {
                rename_type_in_pat(&mut field.pat, old, new);
            }
        }
        syn::Pat::Tuple(pt) => {
            for elem in &mut pt.elems {
                rename_type_in_pat(elem, old, new);
            }
        }
        syn::Pat::TupleStruct(pts) => {
            rename_type_path(&mut pts.path, old, new);
            for elem in &mut pts.elems {
                rename_type_in_pat(elem, old, new);
            }
        }
        syn::Pat::Slice(ps) => {
            for elem in &mut ps.elems {
                rename_type_in_pat(elem, old, new);
            }
        }
        syn::Pat::Paren(pp) => rename_type_in_pat(&mut pp.pat, old, new),
        syn::Pat::Or(po) => {
            for case in &mut po.cases {
                rename_type_in_pat(case, old, new);
            }
        }
        _ => {}
    }
}

fn rename_type_in_expr(expr: &mut syn::Expr, old: &str, new: &str) {
    match expr {
        syn::Expr::Path(ep) => rename_type_path(&mut ep.path, old, new),
        syn::Expr::Call(ec) => {
            rename_type_in_expr(&mut ec.func, old, new);
            for arg in &mut ec.args {
                rename_type_in_expr(arg, old, new);
            }
        }
        syn::Expr::MethodCall(em) => {
            rename_type_in_expr(&mut em.receiver, old, new);
            for arg in &mut em.args {
                rename_type_in_expr(arg, old, new);
            }
        }
        syn::Expr::Block(eb) => rename_type_in_block(&mut eb.block, old, new),
        syn::Expr::Let(el) => rename_type_in_expr(&mut el.expr, old, new),
        syn::Expr::If(ei) => {
            rename_type_in_expr(&mut ei.cond, old, new);
            rename_type_in_block(&mut ei.then_branch, old, new);
            if let Some((_, else_branch)) = &mut ei.else_branch {
                rename_type_in_expr(else_branch, old, new);
            }
        }
        syn::Expr::Match(em) => {
            rename_type_in_expr(&mut em.expr, old, new);
            for arm in &mut em.arms {
                if let Some(guard) = &mut arm.guard {
                    rename_type_in_expr(&mut guard.1, old, new);
                }
                rename_type_in_expr(&mut arm.body, old, new);
            }
        }
        syn::Expr::Closure(ec) => {
            for input in &mut ec.inputs {
                if let syn::Pat::Type(pt) = input {
                    rename_type_in_type(&mut pt.ty, old, new);
                }
            }
            if let syn::ReturnType::Type(_, ty) = &mut ec.output {
                rename_type_in_type(ty, old, new);
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
        syn::Expr::Return(er) => {
            if let Some(e) = &mut er.expr {
                rename_type_in_expr(e, old, new);
            }
        }
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
            rename_type_path(&mut es.path, old, new);
            for field in &mut es.fields {
                rename_type_in_expr(&mut field.expr, old, new);
            }
            if let Some(rest) = &mut es.rest {
                rename_type_in_expr(rest, old, new);
            }
        }
        syn::Expr::Array(ea) => {
            for elem in &mut ea.elems {
                rename_type_in_expr(elem, old, new);
            }
        }
        syn::Expr::Tuple(et) => {
            for elem in &mut et.elems {
                rename_type_in_expr(elem, old, new);
            }
        }
        syn::Expr::Paren(ep) => rename_type_in_expr(&mut ep.expr, old, new),
        syn::Expr::Index(ei) => {
            rename_type_in_expr(&mut ei.expr, old, new);
            rename_type_in_expr(&mut ei.index, old, new);
        }
        syn::Expr::Field(ef) => rename_type_in_expr(&mut ef.base, old, new),
        syn::Expr::Range(er) => {
            if let Some(s) = &mut er.start {
                rename_type_in_expr(s, old, new);
            }
            if let Some(e) = &mut er.end {
                rename_type_in_expr(e, old, new);
            }
        }
        syn::Expr::Cast(ec) => {
            rename_type_in_expr(&mut ec.expr, old, new);
            rename_type_in_type(&mut ec.ty, old, new);
        }
        syn::Expr::Repeat(er) => {
            rename_type_in_expr(&mut er.expr, old, new);
            rename_type_in_expr(&mut er.len, old, new);
        }
        syn::Expr::Break(eb) => {
            if let Some(e) = &mut eb.expr {
                rename_type_in_expr(e, old, new);
            }
        }
        syn::Expr::Unsafe(eu) => rename_type_in_block(&mut eu.block, old, new),
        syn::Expr::Await(ea) => rename_type_in_expr(&mut ea.base, old, new),
        syn::Expr::Try(et) => rename_type_in_expr(&mut et.expr, old, new),
        syn::Expr::Async(ea) => rename_type_in_block(&mut ea.block, old, new),
        syn::Expr::Const(ec) => rename_type_in_block(&mut ec.block, old, new),
        syn::Expr::Lit(_) | syn::Expr::Macro(_) | syn::Expr::Continue(_) => {}
        _ => {}
    }
}

// ── Individual operations ────────────────────────────────────────────────────

pub fn replace_fn_body(file: &mut syn::File, name: &str, new_body: &str) -> Result<bool, String> {
    let body: syn::Block = syn::parse_str(&format!("{{ {new_body} }}"))
        .map_err(|e| format!("invalid body: {e}"))?;
    for item in &mut file.items {
        if let syn::Item::Fn(func) = item {
            if func.sig.ident == name {
                func.block = Box::new(body);
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn add_fn(file: &mut syn::File, name: &str, args: &str, ret: &str, body: &str) -> Result<(), String> {
    let mut inputs = Punctuated::new();
    if !args.trim().is_empty() {
        for part in args.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let arg: syn::FnArg = syn::parse_str(part)
                .map_err(|e| format!("invalid arg '{part}': {e}"))?;
            inputs.push(arg);
        }
    }
    let ret_type = if ret.is_empty() {
        syn::ReturnType::Default
    } else {
        let ty: syn::Type = syn::parse_str(ret)
            .map_err(|e| format!("invalid ret: {e}"))?;
        syn::ReturnType::Type(Default::default(), Box::new(ty))
    };
    let body_block: syn::Block = syn::parse_str(body)
        .map_err(|e| format!("invalid body: {e}"))?;
    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
    let new_fn = syn::ItemFn {
        attrs: vec![],
        vis: syn::Visibility::Public(syn::token::Pub::default()),
        sig: syn::Signature {
            constness: None,
            asyncness: None,
            unsafety: None,
            abi: None,
            fn_token: Default::default(),
            ident,
            generics: Default::default(),
            paren_token: Default::default(),
            inputs,
            variadic: None,
            output: ret_type,
        },
        block: Box::new(body_block),
    };
    file.items.push(syn::Item::Fn(new_fn));
    Ok(())
}

pub fn remove_fn(file: &mut syn::File, name: &str) -> bool {
    let before = file.items.len();
    file.items.retain(|item| {
        if let syn::Item::Fn(func) = item {
            func.sig.ident != name
        } else {
            true
        }
    });
    file.items.len() < before
}

pub fn add_use(file: &mut syn::File, use_path: &str) -> Result<(), String> {
    let use_item: syn::ItemUse = syn::parse_str(&format!("pub use {use_path};"))
        .map_err(|e| format!("invalid use: {e}"))?;
    file.items.insert(0, syn::Item::Use(use_item));
    Ok(())
}

pub fn add_derive(file: &mut syn::File, name: &str, derive: &str) -> Result<bool, String> {
    let new_paths: Vec<syn::Path> = derive
        .split(',')
        .map(|d| syn::parse_str(d.trim()).map_err(|e| format!("invalid derive '{d}': {e}")))
        .collect::<Result<_, _>>()?;
    for item in &mut file.items {
        match item {
            syn::Item::Struct(s) if s.ident == name => {
                let existing = collect_derives(&s.attrs);
                let existing_names: HashSet<String> = existing
                    .iter()
                    .filter_map(|p| p.segments.last().map(|s| s.ident.to_string()))
                    .collect();
                let all: Punctuated<syn::Path, syn::Token![,]> = existing
                    .into_iter()
                    .chain(new_paths.into_iter().filter(|p| {
                        p.segments
                            .last()
                            .map(|s| !existing_names.contains(&s.ident.to_string()))
                            .unwrap_or(true)
                    }))
                    .collect();
                s.attrs.retain(|a| !a.path().is_ident("derive"));
                s.attrs.push(syn::parse_quote!(#[derive(#all)]));
                return Ok(true);
            }
            syn::Item::Enum(e) if e.ident == name => {
                let existing = collect_derives(&e.attrs);
                let existing_names: HashSet<String> = existing
                    .iter()
                    .filter_map(|p| p.segments.last().map(|s| s.ident.to_string()))
                    .collect();
                let all: Punctuated<syn::Path, syn::Token![,]> = existing
                    .into_iter()
                    .chain(new_paths.into_iter().filter(|p| {
                        p.segments
                            .last()
                            .map(|s| !existing_names.contains(&s.ident.to_string()))
                            .unwrap_or(true)
                    }))
                    .collect();
                e.attrs.retain(|a| !a.path().is_ident("derive"));
                e.attrs.push(syn::parse_quote!(#[derive(#all)]));
                return Ok(true);
            }
            _ => {}
        }
    }
    Ok(false)
}

fn collect_derives(attrs: &[syn::Attribute]) -> Vec<syn::Path> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .flat_map(|a| {
            a.parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                .map(|p| p.into_iter().collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .collect()
}

pub fn new_file(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        return Err(format!("file already exists: {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating dir {}: {e}", parent.display()))?;
    }
    if !content.is_empty() {
        syn::parse_file(content)
            .map_err(|e| format!("invalid rust content: {e}"))?;
    }
    std::fs::write(path, content)
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

pub fn remove_file(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path)
        .map_err(|e| format!("removing {}: {e}", path.display()))
}

pub fn list_file(path: &Path) -> Result<Vec<String>, String> {
    let f = parse_file(path)?;
    let mut items = Vec::new();
    for item in &f.items {
        match item {
            syn::Item::Fn(func) => {
                let args: Vec<String> =
                    func.sig.inputs.iter().map(|a| quote::quote!(#a).to_string()).collect();
                let ret = match &func.sig.output {
                    syn::ReturnType::Default => "()".to_string(),
                    syn::ReturnType::Type(_, ty) => quote::quote!(#ty).to_string(),
                };
                items.push(format!("fn {}({}) -> {}", func.sig.ident, args.join(", "), ret));
            }
            syn::Item::Struct(s) => items.push(format!("struct {}", s.ident)),
            syn::Item::Enum(e) => items.push(format!("enum {}", e.ident)),
            syn::Item::Impl(i) => items.push(format!("impl {}", quote::quote!(#i.self_ty))),
            syn::Item::Mod(m) => items.push(format!("mod {}", m.ident)),
            syn::Item::Use(u) => items.push(format!("use {}", quote::quote!(u))),
            syn::Item::Const(c) => items.push(format!("const {}", c.ident)),
            syn::Item::Static(s) => items.push(format!("static {}", s.ident)),
            syn::Item::Trait(t) => items.push(format!("trait {}", t.ident)),
            syn::Item::Type(t) => items.push(format!("type {}", t.ident)),
            syn::Item::ExternCrate(e) => items.push(format!("extern crate {}", e.ident)),
            syn::Item::Union(u) => items.push(format!("union {}", u.ident)),
            _ => {}
        }
    }
    Ok(items)
}

pub fn find_fn(file: &syn::File, name: &str) -> Option<String> {
    for item in &file.items {
        if let syn::Item::Fn(func) = item {
            if func.sig.ident == name {
                let out = syn::File {
                    shebang: None,
                    attrs: vec![],
                    items: vec![item.clone()],
                };
                return Some(pretty_unparse(&out));
            }
        }
    }
    None
}

/// Count incoming refs to identifiers defined in `file` from all other project files.
pub fn incoming_refs(project_files: &[PathBuf], file: &Path) -> usize {
    let Ok(content) = std::fs::read_to_string(file) else { return 0 };
    let Ok(parsed) = syn::parse_file(&content) else { return 0 };

    let defined: HashSet<String> = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(f) => Some(f.sig.ident.to_string()),
            syn::Item::Struct(s) => Some(s.ident.to_string()),
            syn::Item::Enum(e) => Some(e.ident.to_string()),
            syn::Item::Trait(t) => Some(t.ident.to_string()),
            syn::Item::Const(c) => Some(c.ident.to_string()),
            syn::Item::Mod(m) => Some(m.ident.to_string()),
            _ => None,
        })
        .collect();

    if defined.is_empty() {
        return 0;
    }

    let mut total = 0;
    for other in project_files {
        if other == file {
            continue;
        }
        let Ok(c) = std::fs::read_to_string(other) else { continue };
        let Ok(p) = syn::parse_file(&c) else { continue };
        for item in &p.items {
            total += count_refs_to_set(item, &defined);
        }
    }
    total
}

fn count_refs_to_set(item: &syn::Item, defined: &HashSet<String>) -> usize {
    let mut count = 0;
    match item {
        syn::Item::Use(u) => count += count_use_refs(&u.tree, defined),
        syn::Item::Fn(f) => {
            for input in &f.sig.inputs {
                if let syn::FnArg::Typed(pat) = input {
                    count += count_type_refs(&pat.ty, defined);
                }
            }
            if let syn::ReturnType::Type(_, ty) = &f.sig.output {
                count += count_type_refs(ty, defined);
            }
        }
        syn::Item::Struct(s) => {
            for field in &s.fields {
                count += count_type_refs(&field.ty, defined);
            }
        }
        syn::Item::Enum(e) => {
            for variant in &e.variants {
                for field in &variant.fields {
                    count += count_type_refs(&field.ty, defined);
                }
            }
        }
        _ => {}
    }
    count
}

fn count_use_refs(tree: &syn::UseTree, defined: &HashSet<String>) -> usize {
    let mut count = 0;
    match tree {
        syn::UseTree::Path(up) => {
            if defined.contains(&up.ident.to_string()) { count += 1; }
            count += count_use_refs(&up.tree, defined);
        }
        syn::UseTree::Name(un) => {
            if defined.contains(&un.ident.to_string()) { count += 1; }
        }
        syn::UseTree::Rename(urn) => {
            if defined.contains(&urn.rename.to_string()) { count += 1; }
        }
        syn::UseTree::Group(ug) => {
            for child in &ug.items {
                count += count_use_refs(child, defined);
            }
        }
        _ => {}
    }
    count
}

fn count_type_refs(ty: &syn::Type, defined: &HashSet<String>) -> usize {
    let mut count = 0;
    match ty {
        syn::Type::Path(tp) => {
            for seg in &tp.path.segments {
                if defined.contains(&seg.ident.to_string()) { count += 1; }
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner) = arg {
                            count += count_type_refs(inner, defined);
                        }
                    }
                }
            }
        }
        syn::Type::Reference(tr) => count += count_type_refs(&tr.elem, defined),
        syn::Type::Slice(ts) => count += count_type_refs(&ts.elem, defined),
        syn::Type::Array(ta) => count += count_type_refs(&ta.elem, defined),
        syn::Type::Tuple(tt) => {
            for elem in &tt.elems {
                count += count_type_refs(elem, defined);
            }
        }
        _ => {}
    }
    count
}
