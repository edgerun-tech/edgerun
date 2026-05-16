//! Proc macros for the EdgeRun Tokio-compatible facade.

extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_async_test(item).unwrap_or_else(|error| error)
}

fn expand_async_test(item: TokenStream) -> Result<TokenStream, TokenStream> {
    let source = item.to_string();
    let async_fn = source
        .find("async fn")
        .ok_or_else(|| compile_error("edgerun_tokio::test can only be used on async fn items"))?;
    let before_async = source[..async_fn].trim();
    let after_async = &source[async_fn + "async ".len()..];
    let body_start = after_async
        .find('{')
        .ok_or_else(|| compile_error("edgerun_tokio::test could not find function body"))?;
    let signature = after_async[..body_start].trim();
    let body = &after_async[body_start..];
    let name_start = signature
        .find("fn")
        .map(|index| index + "fn".len())
        .ok_or_else(|| compile_error("edgerun_tokio::test could not find function name"))?;
    let after_fn = signature[name_start..].trim_start();
    let name_end = after_fn
        .find('(')
        .ok_or_else(|| compile_error("edgerun_tokio::test could not find parameter list"))?;
    let name = after_fn[..name_end].trim();
    let params_end = after_fn
        .find(')')
        .ok_or_else(|| compile_error("edgerun_tokio::test could not find parameter list end"))?;
    let return_type = after_fn[params_end + 1..].trim();

    let expanded = format!(
        "{before_async}\n#[test]\nfn {name}() {return_type} {{\n    let runtime = ::edgerun_tokio::runtime::Builder::new_current_thread()\n        .enable_all()\n        .build()\n        .expect(\"edgerun tokio test runtime\");\n    runtime.block_on(async move {body})\n}}"
    );
    expanded
        .parse()
        .map_err(|_| compile_error("edgerun_tokio::test generated invalid tokens"))
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .expect("compile_error token stream")
}
