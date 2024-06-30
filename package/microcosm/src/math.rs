use crate::{
    error::{Res, WrapErr, Error},
    schema::cw_serde,
    std::Deps,
    utility::Validate,
};
use serde::Serialize;
use std::str::FromStr;

#[cw_serde]
pub enum Range<T: Serialize + FromStr> {
    Inclusive { low: T, high: T },
}

// ======= MESSAGES =======
#[cw_serde]
pub enum RangeMsg {
    Inclusive { low: String, high: String },
}

impl<T> Validate<Range<T>> for RangeMsg
where
    T: Serialize + FromStr,
    <T as FromStr>::Err: ToString,
{
    fn validate(&self, _deps: Deps) -> Res<Range<T>> {
        Ok(match self {
            RangeMsg::Inclusive { low, high } => Range::Inclusive {
                high: high.parse().map_err(Error::wrap_err)?,
                low: low.parse().map_err(Error::wrap_err)?,
            },
        })
    }
}

impl<T: Serialize + FromStr + ToString> From<Range<T>> for RangeMsg {
    fn from(range: Range<T>) -> RangeMsg {
        match range {
            Range::Inclusive { low, high } => RangeMsg::Inclusive {
                low: low.to_string(),
                high: high.to_string(),
            },
        }
    }
}
