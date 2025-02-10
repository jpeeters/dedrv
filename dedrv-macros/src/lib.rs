use proc_macro::TokenStream;

mod class;
mod device;
mod helpers;

#[proc_macro_attribute]
pub fn class(args: TokenStream, item: TokenStream) -> TokenStream {
    class::run(args.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn device(args: TokenStream, item: TokenStream) -> TokenStream {
    device::run(args.into(), item.into()).into()
}
