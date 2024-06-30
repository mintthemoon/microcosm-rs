use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DataEnum, DeriveInput, Error, Variant, punctuated::Punctuated, token::Comma};
use std::collections::BTreeMap;

fn variants() -> Vec<Variant> {
    vec![
        parse_quote! {
            #[error("Internal error: {0}")]
            Generic(String)
        },
        parse_quote! {
            #[error(transparent)]
            Std(#[from] ::microcosm::std::StdError)
        },
        parse_quote! {
            #[error("Unauthorized to perform this action")]
            Unauthorized
        },
        parse_quote! {
            #[error("Disabled action")]
            Disabled
        },
        parse_quote! {
            #[error("Expired {0}")]
            Expired(&'static str)
        },
        parse_quote! {
            #[error("Insufficient funds provided")]
            InsufficientFunds
        },
        parse_quote! {
            #[error("Funds not accepted for this action")]
            FundsNotAccepted
        },
        parse_quote! {
            #[error("Input provided was invalid")]
            Input
        },
        parse_quote! {
            #[error("{0} not found")]
            NotFound(&'static str)
        },
        parse_quote! {
            #[error("Failed to parse value")]
            Parse
        },
        parse_quote! {
            #[error("Unexpected error")]
            Unexpected
        },
    ]
}

pub fn cw_error(input: DeriveInput) -> TokenStream {
    if let Data::Enum(ref data) = input.data {
        let mut variants_map: BTreeMap<String, Variant> = data
            .variants
            .iter()
            .map(|v| (v.ident.to_string(), v.clone()))
            .collect();
        if variants_map.len() != 0 {
            // allow cw_errors to convert from the base LibraryError
            variants_map.insert("Microcosm".to_string(), parse_quote! {
                #[error(transparent)]
                Microcosm(#[from] ::microcosm::LibraryError)
            });
        }
        for v in variants() {
            let name = v.ident.to_string();
            if !variants_map.contains_key(&name) {
                variants_map.insert(v.ident.to_string(), v);
            }
        }
        let new_variants: Punctuated<Variant, Comma> = variants_map
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        let new_data = Data::Enum(DataEnum { variants: new_variants, ..data.clone() });
        let ident = &input.ident;
        let mut output = input.clone();
        output.data = new_data;
        quote! {
            #[derive(::microcosm::thiserror::Error, ::std::fmt::Debug)]
            #output

            impl Into<::microcosm::std::StdError> for #ident {
                fn into(self) -> ::microcosm::std::StdError {
                    match self {
                        #ident::Std(e) => e,
                        _ => ::microcosm::std::StdError::GenericErr { msg: self.to_string() },
                    }
                }
            }
            
            impl From<::microcosm::std::CoinsError> for #ident {
                fn from(e: ::microcosm::std::CoinsError) -> Self {
                    #ident::Std(e.into())
                }
            }

            impl <T: ToString> ::microcosm::error::WrapErr<T> for #ident {
                fn wrap_err(inner: T) -> Self {
                    #ident::Generic(inner.to_string())
                }
            }
        }
    } else {
        Error::new_spanned(input, "microcosm_error can only be applied to enum types")
            .into_compile_error()
    }
}
