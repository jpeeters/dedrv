use std::fmt::Debug;

use darling::export::NestedMeta;
use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemStatic;

#[derive(Debug, Default, FromMeta)]
struct Args {
    #[darling(default)]
    path: Option<String>,
}

use crate::helpers::{error, token_stream_with_error};

pub fn run(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut errors = TokenStream::new();

    let var: ItemStatic = match syn::parse2(item.clone()) {
        Ok(x) => x,
        Err(e) => return token_stream_with_error(item, e),
    };

    // Static variable identifier and type.
    let ident = var.ident.clone();
    let ty = var.ty.clone();

    // Ensure that the variable name is uppercase, as required by Rust for static variables.
    if ident != ident.to_string().to_uppercase() {
        error(&mut errors, &item, "device variable name must be uppercase");
    }

    // Parse the macro arguments.
    let args = match NestedMeta::parse_meta_list(args.clone()) {
        Ok(x) => x,
        Err(e) => return token_stream_with_error(args, e),
    };

    let args = match Args::from_list(&args) {
        Ok(x) => x,
        Err(e) => {
            errors.extend(e.write_errors());
            Args::default()
        }
    };

    if args.path.is_none() {
        error(&mut errors, &item, "missing device path on instance");
    }

    // Extract the path from arguments. In case of error, the path is "undefined".
    let path = args.path.unwrap_or_default();

    let desc_mod_ident = format_ident!("__dedrv_desc_{}", ident.to_string().to_lowercase());
    let desc_sname = format!(".dedrv.device.{}", ident.to_string().to_lowercase());
    let desc_ident = format_ident!("__DEDRV_DESC_{}", ident);

    quote! {
        // The original device instance variable.
        #item

        // The descriptor module with self-contained imports.
        mod #desc_mod_ident {
            use ::dedrv::{Device, Descriptor};

            use super::*;

            // Do not mangle the function name, so one can debug it easily.
            #[no_mangle]
            fn __dedrv_desc_init(ptr: *const ()) {
                let device: &'static _ = unsafe { &*(ptr as *const #ty) };
                device.init();
            }

            #[allow(unused)]
            #[link_section = #desc_sname]
            static #desc_ident: Descriptor = Descriptor::new(#path, & #ident, __dedrv_desc_init);
        }

        // Compilation errors.
        #errors
    }
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;

    use super::*;

    #[test]
    fn it_should_install_device() -> googletest::Result<()> {
        let code = run(
            quote!(path = "/gpio0"),
            quote! {
                static DEVICE: Device<DriverImpl> = Device::new();
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();
        verify_that!(result, not(contains_substring("error")))?;

        verify_that!(
            result,
            contains_substring(
                quote!(static DEVICE: Device<DriverImpl> = Device::new()).to_string()
            )
        )?;

        verify_that!(
            result,
            contains_substring(quote!(mod __dedrv_desc_device).to_string())
        )?;

        verify_that!(
            result,
            contains_substring(
                quote!(Descriptor::new("/gpio0", &DEVICE, __dedrv_desc_init)).to_string()
            )
        )?;

        Ok(())
    }
}
