use std::fmt::Display;

use proc_macro2::TokenStream;

use quote::{quote, ToTokens};
use syn::{FnArg, ItemTrait, Pat, TraitItem, TraitItemFn};

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

    #[error("class method must not be async")]
    AsyncNotSupported,

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
        // The original device class trait.
        #item

        // The device driver class trait.
        #driver

        // The tag associated with the device class.
        #tag

        // The device accessor implementation for device class trait.
        #impls

        // The errors returned by the present macro.
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
        // The driver module for isolating the device class trait from the driver point of view.
        // Then apply the same visibility as for the original device class trait.
        #visibility mod driver {
            use ::dedrv::{Device, Driver, StateLock};
            use super::*;

            pub trait #ident : Driver {
                #(#fns)*
            }
        }

        // The errors returned by the present macro.
        #errors
    })
}

fn class_driver_method_quote(m: &TraitItemFn) -> Result<TokenStream> {
    validate_method(m)?;

    let ident = m.sig.ident.clone();
    let out = m.sig.output.clone();

    let args: Vec<_> = m.sig.inputs.iter().skip(1).collect();

    let args = if args.is_empty() {
        quote!(state: &StateLock<Self>)
    } else {
        quote!(state: &StateLock<Self>, #(#args),*)
    };

    let params = m.sig.generics.params.clone();
    let r#where = m.sig.generics.where_clause.clone();

    let generics = if params.is_empty() {
        quote!()
    } else {
        quote!(< #params >)
    };

    Ok(quote! {
        fn #ident #generics (#args) #out #r#where;
    })
}

fn class_tag_quote(t: &ItemTrait) -> TokenStream {
    let ident = t.ident.clone();
    let visibility = t.vis.clone();

    quote! {
        pub mod tag {
            #visibility struct #ident;
        }
    }
}

fn class_accessor_impl_quote(t: &ItemTrait) -> TokenStream {
    let mut errors = TokenStream::new();

    let fns = t.items.iter().fold(Vec::new(), |mut acc, x| {
        if let TraitItem::Fn(f) = x {
            acc.push(f);
        }
        acc
    });

    let ident = t.ident.clone();

    let fns: Vec<_> = fns
        .iter()
        .map(|&f| match class_accessor_impl_method_quote(f) {
            Ok(m) => m,
            Err(e) => {
                error(&mut errors, f, e);
                quote!()
            }
        })
        .collect();

    quote! {
        impl<D: driver:: #ident> #ident for Accessor<'_, D, tag:: #ident> {
            #(#fns)*
        }
    }
}

fn class_accessor_impl_method_quote(m: &TraitItemFn) -> Result<TokenStream> {
    validate_method(m)?;

    let ident = m.sig.ident.clone();
    let out = m.sig.output.clone();

    // These are input arguments, which a simple copy from the trait.
    let args = m.sig.inputs.clone();

    // Then, these inputs are converted to a list of identifier to pass through the driver
    // implementation.
    let argv: Vec<_> = m
        .sig
        .inputs
        .iter()
        // First, skip the function receiver.
        .skip(1)
        // Then, map each input argument to its identifier.
        .map(|x| {
            if let FnArg::Typed(t) = x {
                if let Pat::Ident(x) = &*t.pat {
                    return x.ident.clone();
                }
            }
            // Because we skipped the first receiver argument, others must be typed ones.
            unreachable!()
        })
        .collect();

    // Replace the receiver argument with the driver internal state, which is behind a
    // `Mutex<RefCell<D::StateType>>`. So, thanks to internior mutability of the `RefCell`, we can
    // pass the argument as an immutable reference.
    let argv = if argv.is_empty() {
        quote!(&self.inner().state)
    } else {
        quote!(&self.inner().state, #(#argv),*)
    };

    let params = m.sig.generics.params.clone();
    let r#where = m.sig.generics.where_clause.clone();

    let generics = if params.is_empty() {
        quote!()
    } else {
        quote!(< #params >)
    };

    Ok(quote! {
        fn #ident #generics (#args) #out #r#where {
            // Call the driver implementation of the device class trait.
            D:: #ident (#argv)
        }
    })
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

    if m.sig.asyncness.is_some() {
        return Err(Error::AsyncNotSupported);
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
        verify_that!(
            result,
            contains_substring(
                quote!(
                    trait SomeClass {}
                )
                .to_string()
            )
        )?;

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
            contains_substring(quote!(fn a_method(state: &StateLock<Self>)).to_string())
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
            contains_substring(quote!(fn a_method(state: &StateLock<Self>, arg: u32)).to_string())
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
            contains_substring(quote!(fn a_method(state: &StateLock<Self>) -> u32).to_string())
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
            contains_substring(quote!(fn a_method(state: &StateLock<Self>)).to_string())
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
            contains_substring(quote!(fn a_method<T>(state: &StateLock<Self>)).to_string())
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
            contains_substring(quote!(fn a_method<T1, T2>(state: &StateLock<Self>)).to_string())
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
                quote!(fn a_method<T>(state: &StateLock<Self>) where T: Default).to_string()
            )
        )?;

        Ok(())
    }
}
