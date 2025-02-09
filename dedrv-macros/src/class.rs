use std::fmt::Display;

use proc_macro2::TokenStream;

use quote::{quote, ToTokens};
use syn::{FnArg, ImplItemFn, ItemTrait, TraitItem, TraitItemFn};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, Default, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("class trait must not be generic")]
    InvalidClassGenerics,

    #[error("class trait must not have a where clause")]
    InvalidClassWhereClause,

    #[error("class method must have a self receiver")]
    MissingReceiver,

    #[default]
    #[error("undefined error")]
    Undefined,
}

fn token_stream_with_error(mut tokens: TokenStream, err: syn::Error) -> TokenStream {
    tokens.extend(err.into_compile_error());
    tokens
}

fn error<A: ToTokens, T: Display>(tokens: &mut TokenStream, obj: A, msg: T) {
    tokens.extend(syn::Error::new_spanned(obj.into_token_stream(), msg).into_compile_error())
}

pub fn run(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut errors = TokenStream::new();

    if !args.is_empty() {
        error(&mut errors, &args, "no attribute options supported");
    }

    let t: ItemTrait = match syn::parse2(item.clone()) {
        Ok(x) => x,
        Err(e) => return token_stream_with_error(item, e),
    };

    let driver = match class_driver_quote(&t) {
        Ok(d) => d,
        Err(e) => {
            error(&mut errors, &t, e);
            quote!()
        }
    };

    let tag = class_tag_quote(&t);
    let impls = class_accessor_impl_quote(&t);

    quote! {
        #item
        #driver
        #tag
        #impls
        #errors
    }
}

fn class_driver_quote(t: &ItemTrait) -> Result<TokenStream> {
    validate_trait(t)?;

    let mut errors = TokenStream::new();

    let fns = t.items.iter().fold(Vec::new(), |mut acc, x| {
        if let TraitItem::Fn(f) = x {
            acc.push(f);
        }
        acc
    });

    let ident = t.ident.clone();
    let visibility = t.vis.clone();

    let fns: Vec<_> = fns
        .iter()
        .map(|&f| match class_driver_method_quote(f) {
            Ok(m) => m,
            Err(e) => {
                error(&mut errors, f, e);
                quote!()
            }
        })
        .collect();

    Ok(quote! {
        mod driver {
            use ::dedrv::{Device, Driver};
            use super::*;

            #visibility trait #ident : Driver {
                #(#fns)*
            }
        }

        #errors
    })
}

fn class_driver_method_quote(m: &TraitItemFn) -> Result<TokenStream> {
    validate_method(m)?;

    let ident = m.sig.ident.clone();
    let out = m.sig.output.clone();

    let mutable = if let Some(arg) = m.sig.inputs.first() {
        match arg {
            FnArg::Receiver(r) => r.mutability.is_some(),
            _ => false,
        }
    } else {
        return Err(Error::MissingReceiver);
    };

    let args: Vec<_> = m.sig.inputs.iter().skip(1).collect();
    let mutability = if mutable { quote!(mut) } else { quote!() };

    let args = if args.is_empty() {
        quote!(dev: & #mutability Self::StateType)
    } else {
        quote!(dev: & #mutability Self::StateType, #(#args),*)
    };

    let r#async = m.sig.asyncness;
    let params = m.sig.generics.params.clone();
    let r#where = m.sig.generics.where_clause.clone();

    let generics = if params.is_empty() {
        quote!()
    } else {
        quote!(< #params >)
    };

    Ok(quote! {
        #r#async fn #ident #generics (#args) #out #r#where;
    })
}

fn class_tag_quote(t: &ItemTrait) -> TokenStream {
    let ident = t.ident.clone();

    quote! {
        mod tag {
            struct #ident;
        }
    }
}

fn class_accessor_impl_quote(t: &ItemTrait) -> TokenStream {
    let ident = t.ident.clone();
    let fns: Vec<ImplItemFn> = Vec::new();

    quote! {
        impl<D: driver:: #ident> #ident for Accessor<'_, D, tag:: #ident> {
            #(#fns)*
        }
    }
}

fn validate_trait(t: &ItemTrait) -> Result<()> {
    if !t.generics.params.is_empty() {
        return Err(Error::InvalidClassGenerics);
    }

    if t.generics.where_clause.is_some() {
        return Err(Error::InvalidClassWhereClause);
    }

    Ok(())
}

fn validate_method(m: &TraitItemFn) -> Result<()> {
    let arg = match m.sig.inputs.first() {
        Some(x) => x,
        None => return Err(Error::MissingReceiver),
    };

    if !matches!(arg, FnArg::Receiver(_)) {
        return Err(Error::MissingReceiver);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;

    use super::*;

    #[test]
    fn it_should_compile_class_trait() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {}
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(result, contains_substring(quote!(mod driver).to_string()))?;
        verify_that!(result, contains_substring(quote!(SomeClass<D>).to_string()))?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_no_arg() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    fn a_method(&self);
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(quote!(fn a_method(dev: &Device<D>)).to_string())
        )?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_one_arg() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    fn a_method(&self, arg: u32);
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(quote!(fn a_method(dev: &Device<D>, arg: u32)).to_string())
        )?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_no_arg_and_output() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    fn a_method(&self) -> u32;
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(quote!(fn a_method(dev: &Device<D>) -> u32).to_string())
        )?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_mutable_and_no_arg() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    fn a_method(&mut self);
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(quote!(fn a_method(dev: &mut Device<D>)).to_string())
        )?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_async_and_no_arg() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    async fn a_method(&self);
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(quote!(async fn a_method(dev: &Device<D>)).to_string())
        )?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_one_param_and_no_arg() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    fn a_method<T>(&self);
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(quote!(fn a_method<T>(dev: &Device<D>)).to_string())
        )?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_two_params_and_no_arg() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    fn a_method<T1, T2>(&self);
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(quote!(fn a_method<T1, T2>(dev: &Device<D>)).to_string())
        )?;

        Ok(())
    }

    #[test]
    fn it_should_compile_method_with_one_param_and_clause_and_no_arg() -> googletest::Result<()> {
        let code = run(
            quote!(),
            quote! {
                trait SomeClass {
                    fn a_method<T>(&self) where T: Default;
                }
            },
        );

        assert_that!(code.is_empty(), eq(false));

        let result = code.to_string();

        verify_that!(result, not(contains_substring("error")))?;
        verify_that!(
            result,
            contains_substring(
                quote!(fn a_method<T>(dev: &Device<D>) where T: Default).to_string()
            )
        )?;

        Ok(())
    }
}
