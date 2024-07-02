use crate::{
    error::{Error, Res, WrapErr},
    math::Range,
    schema::cw_serde,
    std::Uint128,
};
use std::cmp::min;

#[cw_serde]
pub struct PageMsg {
    pub index: Uint128,
    pub end: Uint128,
}

#[cw_serde]
pub struct PageQuery {
    pub index: Uint128,
    pub limit: Option<u32>,
}

pub struct PageLimits {
    pub default: u32,
    pub max: u32,
}

impl PageLimits {
    pub fn page_info(
        &self,
        query: Option<PageQuery>,
        max_items: Uint128,
    ) -> Res<(PageMsg, Range<Uint128>)> {
        let (index, limit) = match query {
            Some(q) => (q.index, q.limit.unwrap_or(self.default)),
            None => (Uint128::zero(), self.default),
        };
        if limit == 0 || limit > self.max {
            return Err(Error::Input {});
        }
        let start = index
            .checked_mul(Uint128::new(limit as u128))
            .map_err(Error::wrap_err)?;
        if start > max_items {
            return Err(Error::Input {});
        }
        let end = min(
            start
                .checked_add(Uint128::new(limit as u128))
                .map_err(Error::wrap_err)?,
            max_items,
        );
        let page = PageMsg { index, end };
        let range = Range { low: start, high: end };
        Ok((page, range))
    }
}
