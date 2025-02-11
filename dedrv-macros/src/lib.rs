#![deny(missing_docs)]

//! This crate defines the macros that are used by `dedrv` for declaring device classes and
//! instances.

use proc_macro::TokenStream;

mod class;
mod device;
mod helpers;

/// The `class` attribute that transforms a trait into a device class.
#[proc_macro_attribute]
pub fn class(args: TokenStream, item: TokenStream) -> TokenStream {
    class::run(args.into(), item.into()).into()
}

/// The `device` attribute that transform a static device instance into a registered device.
#[proc_macro_attribute]
pub fn device(args: TokenStream, item: TokenStream) -> TokenStream {
    device::run(args.into(), item.into()).into()
}
