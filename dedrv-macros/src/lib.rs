use proc_macro::TokenStream;

mod class;

#[proc_macro_attribute]
pub fn class(args: TokenStream, item: TokenStream) -> TokenStream {
    class::run(args.into(), item.into()).into()
}
