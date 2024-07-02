use crate::{
    error::{Res, WrapErr, Error},
    schema::cw_serde,
    std::Deps,
    utility::Validate,
    cw_storage_plus::{Bound, PrimaryKey},
};
use serde::Serialize;
use std::str::FromStr;

#[cw_serde]
pub struct Range<T: Serialize + FromStr + Clone> {
    pub low: T,
    pub high: T,
}

impl <T: Serialize + FromStr + Clone> Range<T> {
    pub fn new(low: T, high: T) -> Self {
        Range { low, high }
    }

    pub fn bounds<'a, U: PrimaryKey<'a> + From<T>>(&self) -> Res<(Bound<'a, U>, Bound<'a, U>)> {
        Ok((
            Bound::inclusive(U::from(self.low.clone())),
            Bound::exclusive(U::from(self.high.clone())),
        ))
    }
}

// ======= MESSAGES =======
#[cw_serde]
pub struct RangeMsg {
    low: String,
    high: String,
}

impl<T> Validate<Range<T>> for RangeMsg
where
    T: Serialize + FromStr + Clone,
    <T as FromStr>::Err: ToString,
{
    fn validate(&self, _deps: Deps) -> Res<Range<T>> {
        Ok(Range::new(
            self.low.parse().map_err(Error::wrap_err)?,
            self.high.parse().map_err(Error::wrap_err)?,
        ))
    }
}

impl<T: Serialize + FromStr + Clone + ToString> From<Range<T>> for RangeMsg {
    fn from(range: Range<T>) -> RangeMsg {
        RangeMsg {
            low: range.low.to_string(),
            high: range.high.to_string(),
        }
    }
}
