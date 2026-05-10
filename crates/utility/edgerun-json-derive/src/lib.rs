use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Field;
use syn::Fields;
use syn::LitStr;
use syn::Token;
use syn::Type;

#[derive(Default)]
struct ContainerAttrs {
    rename_all: Option<String>,
    tag: Option<String>,
}

#[derive(Default)]
struct FieldAttrs {
    rename: Option<String>,
    aliases: Vec<String>,
    default: bool,
    default_fn: Option<String>,
    skip_serializing_if: Option<String>,
    skip_serializing: bool,
    skip_deserializing: bool,
}

fn to_snake_case(value: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if ch.is_uppercase() {
            if idx > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_kebab_case(value: &str) -> String {
    to_snake_case(value).replace('_', "-")
}

fn apply_rename_all(value: &str, rename_all: Option<&str>) -> String {
    match rename_all {
        Some("snake_case") => to_snake_case(value),
        Some("kebab-case") => to_kebab_case(value),
        Some("lowercase") => value.to_ascii_lowercase(),
        Some("UPPERCASE") => value.to_ascii_uppercase(),
        _ => value.to_string(),
    }
}

fn parse_container_attrs(attrs: &[Attribute]) -> ContainerAttrs {
    let mut out = ContainerAttrs::default();
    for attr in attrs {
        if !(attr.path().is_ident("json") || attr.path().is_ident("serde")) {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value = meta.value()?.parse::<LitStr>()?;
                out.rename_all = Some(value.value());
            } else if meta.path.is_ident("tag") {
                let value = meta.value()?.parse::<LitStr>()?;
                out.tag = Some(value.value());
            }
            Ok(())
        });
    }
    out
}

fn parse_field_attrs(attrs: &[Attribute]) -> FieldAttrs {
    let mut out = FieldAttrs::default();
    for attr in attrs {
        if !(attr.path().is_ident("json") || attr.path().is_ident("serde")) {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?.parse::<LitStr>()?;
                out.rename = Some(value.value());
            } else if meta.path.is_ident("alias") {
                let value = meta.value()?.parse::<LitStr>()?;
                out.aliases.push(value.value());
            } else if meta.path.is_ident("default") {
                if meta.input.peek(Token![=]) {
                    let value = meta.value()?.parse::<LitStr>()?;
                    out.default_fn = Some(value.value());
                } else {
                    out.default = true;
                }
            } else if meta.path.is_ident("skip_serializing_if") {
                let value = meta.value()?.parse::<LitStr>()?;
                out.skip_serializing_if = Some(value.value());
            } else if meta.path.is_ident("skip_serializing") {
                out.skip_serializing = true;
            } else if meta.path.is_ident("skip_deserializing") {
                out.skip_deserializing = true;
            }
            Ok(())
        });
    }
    out
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn field_json_key(field: &Field, container: &ContainerAttrs) -> String {
    let attrs = parse_field_attrs(&field.attrs);
    attrs.rename.unwrap_or_else(|| {
        apply_rename_all(
            &field.ident.as_ref().expect("named field").to_string(),
            container.rename_all.as_deref(),
        )
    })
}

fn field_json_keys(field: &Field, container: &ContainerAttrs) -> Vec<String> {
    let attrs = parse_field_attrs(&field.attrs);
    let mut keys = Vec::with_capacity(attrs.aliases.len() + 1);
    keys.push(attrs.rename.unwrap_or_else(|| {
        apply_rename_all(
            &field.ident.as_ref().expect("named field").to_string(),
            container.rename_all.as_deref(),
        )
    }));
    keys.extend(attrs.aliases);
    keys
}

fn derive_to_json_struct(
    input: &DeriveInput,
    fields: &syn::FieldsNamed,
) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let container = parse_container_attrs(&input.attrs);
    let writes = fields.named.iter().map(|field| {
        let ident = field.ident.as_ref().expect("named field");
        let key = field_json_key(field, &container);
        let attrs = parse_field_attrs(&field.attrs);
        if attrs.skip_serializing {
            quote! {}
        } else if attrs.skip_serializing_if.as_deref() == Some("Option::is_none")
            && is_option_type(&field.ty)
        {
            quote! {
                if let Some(value) = &self.#ident {
                    object.push_field(#key, edgerun_json::ToJson::to_json(value));
                }
            }
        } else {
            quote! {
                object.push_field(#key, edgerun_json::ToJson::to_json(&self.#ident));
            }
        }
    });

    quote! {
        impl #impl_generics edgerun_json::ToJson for #name #ty_generics #where_clause {
            fn to_json(&self) -> edgerun_json::JsonValue {
                let mut object = edgerun_json::Map::new();
                #(#writes)*
                edgerun_json::JsonValue::Object(object)
            }
        }
    }
}

fn derive_from_json_struct(
    input: &DeriveInput,
    fields: &syn::FieldsNamed,
) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let container = parse_container_attrs(&input.attrs);
    let reads = fields.named.iter().map(|field| {
        let ident = field.ident.as_ref().expect("named field");
        let keys = field_json_keys(field, &container);
        let attrs = parse_field_attrs(&field.attrs);
        if attrs.skip_deserializing {
            quote! {
                #ident: Default::default()
            }
        } else if let Some(default_fn) = attrs.default_fn {
            match syn::parse_str::<syn::Path>(&default_fn) {
                Ok(default_fn) => quote! {
                    #ident: object.take_optional_any(&[#(#keys),*])?.unwrap_or_else(#default_fn)
                },
                Err(err) => err.to_compile_error(),
            }
        } else if attrs.default {
            quote! {
                #ident: object.take_optional_any(&[#(#keys),*])?.unwrap_or_default()
            }
        } else if is_option_type(&field.ty) {
            quote! {
                #ident: object.take_optional_any(&[#(#keys),*])?
            }
        } else {
            quote! {
                #ident: object.take_required_any(&[#(#keys),*])?
            }
        }
    });

    quote! {
        impl #impl_generics edgerun_json::FromJson for #name #ty_generics #where_clause {
            fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
                let mut object = value.into_object(core::any::type_name::<Self>())?;
                Ok(Self {
                    #(#reads),*
                })
            }
        }
    }
}

fn derive_to_json_enum(input: &DeriveInput, data: &syn::DataEnum) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let container = parse_container_attrs(&input.attrs);
    if let Some(tag) = container.tag.as_ref() {
        let arms = data.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let attrs = parse_field_attrs(&variant.attrs);
            let value = attrs.rename.unwrap_or_else(|| {
                apply_rename_all(&ident.to_string(), container.rename_all.as_deref())
            });
            match &variant.fields {
                Fields::Unit => {
                    quote! {
                        Self::#ident => {
                            let mut object = edgerun_json::Map::new();
                            object.push_field(#tag, edgerun_json::JsonValue::String(#value.to_string()));
                            edgerun_json::JsonValue::Object(object)
                        }
                    }
                }
                Fields::Named(fields) => {
                    let field_idents: Vec<_> = fields
                        .named
                        .iter()
                        .map(|field| field.ident.as_ref().expect("named field"))
                        .collect();
                    let writes = fields.named.iter().map(|field| {
                        let ident = field.ident.as_ref().expect("named field");
                        let key = field_json_key(field, &container);
                        let attrs = parse_field_attrs(&field.attrs);
                        if attrs.skip_serializing {
                            quote! {}
                        } else if attrs.skip_serializing_if.as_deref() == Some("Option::is_none")
                            && is_option_type(&field.ty)
                        {
                            quote! {
                                if let Some(value) = #ident {
                                    object.push_field(#key, edgerun_json::ToJson::to_json(value));
                                }
                            }
                        } else {
                            quote! {
                                object.push_field(#key, edgerun_json::ToJson::to_json(#ident));
                            }
                        }
                    });
                    quote! {
                        Self::#ident { #(#field_idents),* } => {
                            let mut object = edgerun_json::Map::new();
                            object.push_field(#tag, edgerun_json::JsonValue::String(#value.to_string()));
                            #(#writes)*
                            edgerun_json::JsonValue::Object(object)
                        }
                    }
                }
                Fields::Unnamed(_) => quote! {
                    Self::#ident(..) => compile_error!("ToJson does not support tuple variants")
                },
            }
        });

        return quote! {
            impl #impl_generics edgerun_json::ToJson for #name #ty_generics #where_clause {
                fn to_json(&self) -> edgerun_json::JsonValue {
                    match self {
                        #(#arms,)*
                    }
                }
            }
        };
    }

    let arms = data.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let attrs = parse_field_attrs(&variant.attrs);
        let value = attrs.rename.unwrap_or_else(|| {
            apply_rename_all(&ident.to_string(), container.rename_all.as_deref())
        });
        match &variant.fields {
            Fields::Unit => quote! { Self::#ident => #value },
            _ => quote! { Self::#ident { .. } => #value },
        }
    });

    quote! {
        impl #impl_generics edgerun_json::ToJson for #name #ty_generics #where_clause {
            fn to_json(&self) -> edgerun_json::JsonValue {
                edgerun_json::JsonValue::String(match self {
                    #(#arms,)*
                }.to_string())
            }
        }
    }
}

fn derive_from_json_enum(input: &DeriveInput, data: &syn::DataEnum) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let container = parse_container_attrs(&input.attrs);
    if let Some(tag) = container.tag.as_ref() {
        let arms = data.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let attrs = parse_field_attrs(&variant.attrs);
            let value = attrs.rename.unwrap_or_else(|| {
                apply_rename_all(&ident.to_string(), container.rename_all.as_deref())
            });
            match &variant.fields {
                Fields::Unit => quote! {
                    #value => Ok(Self::#ident)
                },
                Fields::Named(fields) => {
                    let reads = fields.named.iter().map(|field| {
                        let ident = field.ident.as_ref().expect("named field");
                        let keys = field_json_keys(field, &container);
                        let attrs = parse_field_attrs(&field.attrs);
                        if attrs.skip_deserializing {
                            quote! {
                                #ident: Default::default()
                            }
                        } else if let Some(default_fn) = attrs.default_fn {
                            match syn::parse_str::<syn::Path>(&default_fn) {
                                Ok(default_fn) => quote! {
                                    #ident: object.take_optional_any(&[#(#keys),*])?.unwrap_or_else(#default_fn)
                                },
                                Err(err) => err.to_compile_error(),
                            }
                        } else if attrs.default {
                            quote! {
                                #ident: object.take_optional_any(&[#(#keys),*])?.unwrap_or_default()
                            }
                        } else if is_option_type(&field.ty) {
                            quote! {
                                #ident: object.take_optional_any(&[#(#keys),*])?
                            }
                        } else {
                            quote! {
                                #ident: object.take_required_any(&[#(#keys),*])?
                            }
                        }
                    });
                    quote! {
                        #value => Ok(Self::#ident {
                            #(#reads),*
                        })
                    }
                }
                Fields::Unnamed(_) => quote! {
                    #value => Err(edgerun_json::JsonValueError::WrongType(
                        edgerun_json::__json_error_message("tuple variants are not supported")
                    ))
                },
            }
        });

        return quote! {
            impl #impl_generics edgerun_json::FromJson for #name #ty_generics #where_clause {
                fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
                    let mut object = value.into_object(core::any::type_name::<Self>())?;
                    let tag = <String as edgerun_json::FromJson>::from_json(
                        object.remove(#tag).ok_or_else(|| {
                            edgerun_json::JsonValueError::WrongType(
                                edgerun_json::__json_error_message("missing JSON tag field")
                            )
                        })?
                    )?;
                    match tag.as_str() {
                        #(#arms,)*
                        _ => Err(edgerun_json::JsonValueError::WrongType(
                            edgerun_json::__json_error_message("unknown enum tag")
                        )),
                    }
                }
            }
        };
    }

    let arms = data.variants.iter().filter_map(|variant| {
        let ident = &variant.ident;
        if !matches!(variant.fields, Fields::Unit) {
            return None;
        }
        let attrs = parse_field_attrs(&variant.attrs);
        let value = attrs.rename.unwrap_or_else(|| {
            apply_rename_all(&ident.to_string(), container.rename_all.as_deref())
        });
        Some(quote! { #value => Ok(Self::#ident) })
    });

    quote! {
        impl #impl_generics edgerun_json::FromJson for #name #ty_generics #where_clause {
            fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
                let value = <String as edgerun_json::FromJson>::from_json(value)?;
                match value.as_str() {
                    #(#arms,)*
                    _ => Err(edgerun_json::JsonValueError::WrongType(
                        edgerun_json::__json_error_message("unknown enum value")
                    )),
                }
            }
        }
    }
}

#[proc_macro_derive(ToJson, attributes(json, serde))]
pub fn derive_to_json(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let output = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => derive_to_json_struct(&input, fields),
            _ => syn::Error::new_spanned(&input.ident, "ToJson only supports named structs")
                .to_compile_error(),
        },
        Data::Enum(data) => derive_to_json_enum(&input, data),
        Data::Union(_) => syn::Error::new_spanned(&input.ident, "ToJson does not support unions")
            .to_compile_error(),
    };
    output.into()
}

#[proc_macro_derive(FromJson, attributes(json, serde))]
pub fn derive_from_json(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let output = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => derive_from_json_struct(&input, fields),
            _ => syn::Error::new_spanned(&input.ident, "FromJson only supports named structs")
                .to_compile_error(),
        },
        Data::Enum(data) => derive_from_json_enum(&input, data),
        Data::Union(_) => syn::Error::new_spanned(&input.ident, "FromJson does not support unions")
            .to_compile_error(),
    };
    output.into()
}
