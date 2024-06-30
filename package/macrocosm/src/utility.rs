use syn::{Ident, Result, parse::{Parse, ParseStream}, punctuated::Punctuated, token::Comma};

// mod unwrap_or_throw {
//     macro_rules! unwrap_or_throw {
//         ($result:expr) => {
//             match $result {
//                 Ok(value) => value,
//                 Err(err) => return err.into_compile_error(),
//             }
//         };
//     }

//     pub(crate) use unwrap_or_throw;
// }

// pub(crate) use unwrap_or_throw::unwrap_or_throw;

pub struct MacroArgs {
    pub vars: Vec<String>
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let vars = Punctuated::<Ident, Comma>::parse_terminated(input)?
            .into_iter()
            .map(|i| i.to_string())
            .collect();
        Ok(MacroArgs { vars })
    }
}