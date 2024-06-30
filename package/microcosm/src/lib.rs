pub mod funds;
pub mod utility;
pub mod math;
pub mod page;
pub mod msg;
mod error;

pub use crate::error::Error as LibraryError;

pub mod schema {
    use std::collections::BTreeMap;
    use schemars::{JsonSchema, schema::RootSchema};
    pub use macrocosm::{cw_serde, QueryResponses};
    pub use cosmwasm_schema::{
        generate_api,
        write_api,
        export_schema,
        export_schema_with_title,
        combine_subqueries,
        remove_schemas,
        Api,
        IntegrityError,
        schema_for,
    };
    
    pub trait QueryResponses: JsonSchema {
        fn response_schemas() -> Result<BTreeMap<String, RootSchema>, IntegrityError> {
            let response_schemas = Self::response_schemas_impl();
            Ok(response_schemas)
        }
        fn response_schemas_impl() -> BTreeMap<String, RootSchema>;
    }
}

pub use cosmwasm_std as std;
pub use cw_storage_plus;
pub use anyhow;
pub use thiserror;
pub use schemars;
pub use serde;

extern crate self as microcosm;