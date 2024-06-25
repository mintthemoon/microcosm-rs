use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Deps;
use serde::Serialize;

use crate::utility::Validate;
use crate::{Res, IntoRes};

#[cw_serde]
pub enum Range<T: Serialize + FromStr> {
    Inclusive { low: T, high: T },
}

// ======= MESSAGES =======
#[cw_serde]
pub enum RangeMsg {
    Inclusive { low: String, high: String },
}

impl <T> Validate<Range<T>> for RangeMsg where T: Serialize + FromStr, <T as FromStr>::Err: std::error::Error + 'static {
    fn validate(&self, _deps: Deps) -> Res<Range<T>> {
        Ok(match self {
            RangeMsg::Inclusive { low, high } => Range::Inclusive{ high: high.parse().res()?, low: low.parse().res()? },
        })
    }
}

impl <T: Serialize + FromStr + ToString> From<Range<T>> for RangeMsg where {
    fn from(range: Range<T>) -> RangeMsg {
        match range {
            Range::Inclusive { low, high } => RangeMsg::Inclusive { low: low.to_string(), high: high.to_string() },
        }
    }
}