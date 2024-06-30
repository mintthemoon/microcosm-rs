use syn::{parse_macro_input, DeriveInput, parse};
use quote::ToTokens;
use proc_macro::TokenStream;

mod serde;
mod query_responses;
mod errors;
mod utility;

#[proc_macro_attribute]
pub fn cw_serde(
    _attr: TokenStream,
    input: TokenStream,
) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let expanded = serde::cw_serde_impl(derive_input).into_token_stream();
    TokenStream::from(expanded)
}

#[proc_macro_derive(QueryResponses, attributes(returns, query_responses))]
pub fn query_responses_derive(
    input: TokenStream,
) -> TokenStream {
    parse(input)
        .map(|item| query_responses::query_responses_derive_impl(item)
            .map(|i| i.into_token_stream())
            .unwrap_or_else(|err| err.into_compile_error())
        )
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn cw_error(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    errors::cw_error(derive_input).into()
}