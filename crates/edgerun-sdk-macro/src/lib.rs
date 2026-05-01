use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_body = &input_fn.block;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;

    let generated = quote! {
        #input_fn

        #[no_mangle]
        pub extern "C" fn run(ptr: i32, len: i32) -> i32 {
            let input_bytes = unsafe {
                core::slice::from_raw_parts(ptr as *const u8, len as usize)
            };

            let req = edgerun_sdk::Request::from_bytes(input_bytes);

            let resp = (|| #fn_output {
                #fn_body
            })();

            let output_bytes = resp.to_bytes();
            let output_ptr = output_bytes.as_ptr() as i32;
            let output_len = output_bytes.len() as i32;

            unsafe {
                edgerun_sdk::host::write_output(output_ptr, output_len);
            }

            core::mem::forget(output_bytes);

            0
        }
    };

    generated.into()
}
