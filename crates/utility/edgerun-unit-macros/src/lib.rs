use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Error, FnArg, ItemFn, LitStr, PatType, ReturnType, Type,
    Visibility,
};

#[proc_macro_attribute]
pub fn export(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        let parsed = parse_macro_input!(attr as LitStr);
        return Error::new_spanned(parsed, "edgerun_unit::export takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut function = parse_macro_input!(item as ItemFn);
    if let Err(err) = validate_export_signature(&function) {
        return err.to_compile_error().into();
    }

    if function
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("no_mangle"))
    {
        return Error::new_spanned(
            function.sig.ident,
            "edgerun_unit::export owns #[no_mangle]; remove the manual attribute",
        )
        .to_compile_error()
        .into();
    }

    function
        .sig
        .abi
        .get_or_insert_with(|| parse_quote!(extern "C"));
    function.attrs.push(parse_quote!(#[no_mangle]));
    function.vis = Visibility::Public(parse_quote!(pub));

    quote!(#function).into()
}

fn validate_export_signature(function: &ItemFn) -> Result<(), Error> {
    if !function.sig.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &function.sig.generics,
            "edgerun unit exports cannot be generic",
        ));
    }
    if function.sig.constness.is_some() {
        return Err(Error::new_spanned(
            function.sig.constness,
            "edgerun unit exports cannot be const functions",
        ));
    }
    if function.sig.asyncness.is_some() {
        return Err(Error::new_spanned(
            function.sig.asyncness,
            "edgerun unit exports cannot be async functions",
        ));
    }
    if function.sig.variadic.is_some() {
        return Err(Error::new_spanned(
            &function.sig.variadic,
            "edgerun unit exports cannot be variadic",
        ));
    }

    match &function.sig.abi {
        Some(abi) => {
            let is_c = abi
                .name
                .as_ref()
                .map(|name| name.value() == "C")
                .unwrap_or(false);
            if !is_c {
                return Err(Error::new_spanned(
                    abi,
                    "edgerun unit exports must use extern \"C\"",
                ));
            }
        }
        None => {}
    }

    for input in &function.sig.inputs {
        let FnArg::Typed(PatType { ty, .. }) = input else {
            return Err(Error::new_spanned(
                input,
                "edgerun unit exports cannot take self receivers",
            ));
        };
        if !is_wasm_scalar_type(ty) {
            return Err(Error::new_spanned(
                ty,
                "edgerun unit export parameters must be i32, i64, f32, or f64",
            ));
        }
    }

    match &function.sig.output {
        ReturnType::Default => {}
        ReturnType::Type(_, ty) => {
            if !is_wasm_scalar_type(ty) {
                return Err(Error::new_spanned(
                    ty,
                    "edgerun unit export return values must be i32, i64, f32, or f64",
                ));
            }
        }
    }

    Ok(())
}

fn is_wasm_scalar_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some() {
        return false;
    }
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    matches!(
        segment.ident.to_string().as_str(),
        "i32" | "i64" | "f32" | "f64"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn accepts_wasm_scalar_signature() {
        let function: ItemFn = parse_quote! {
            unsafe fn digest(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
                0
            }
        };

        assert!(validate_export_signature(&function).is_ok());
    }

    #[test]
    fn rejects_generic_export() {
        let function: ItemFn = parse_quote! {
            fn generic<T>(value: i32) -> i32 {
                value
            }
        };

        let err = validate_export_signature(&function).expect_err("generic");
        assert!(err.to_string().contains("generic"));
    }

    #[test]
    fn rejects_non_c_abi() {
        let function: ItemFn = parse_quote! {
            extern "Rust" fn rust_abi(value: i32) -> i32 {
                value
            }
        };

        let err = validate_export_signature(&function).expect_err("abi");
        assert!(err.to_string().contains("extern \"C\""));
    }

    #[test]
    fn rejects_non_scalar_parameter() {
        let function: ItemFn = parse_quote! {
            fn pointer(value: *const u8) -> i32 {
                0
            }
        };

        let err = validate_export_signature(&function).expect_err("parameter");
        assert!(err.to_string().contains("parameters"));
    }

    #[test]
    fn rejects_non_scalar_return() {
        let function: ItemFn = parse_quote! {
            fn pointer() -> *const u8 {
                core::ptr::null()
            }
        };

        let err = validate_export_signature(&function).expect_err("return");
        assert!(err.to_string().contains("return"));
    }
}
