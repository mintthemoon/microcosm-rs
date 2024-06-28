use syn::{parse_macro_input, DeriveInput};
use quote::ToTokens;

mod serde;

#[proc_macro_attribute]
pub fn cw_serde(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = serde::cw_serde_impl(input).into_token_stream();
    proc_macro::TokenStream::from(expanded)
}