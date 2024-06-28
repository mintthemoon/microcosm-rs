use crate::{
    schema::cw_serde,
    std::Uint128,
    error::{Res, Error},
};

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
    pub default_limit: u32,
    pub max_limit: u32,
    pub max_items: Uint128,
}

impl PageLimits {
    pub fn new(default_limit: u32, max_limit: u32, max_items: Uint128) -> Self {
        PageLimits {
            default_limit,
            max_limit,
            max_items,
        }
    }

    pub fn start_index(&self, page: PageQuery) -> Res<Uint128> {
        let limit = page.limit.unwrap_or(self.default_limit);
        if limit > self.max_limit {
            return Err(Error::Input {});
        }
        let start = page.index * Uint128::new(limit as u128);
        if start >= self.max_items {
            Err(Error::Input {})
        } else {
            Ok(start)
        }
    }

    pub fn end_index(&self, page: PageQuery) -> Res<Uint128> {
        let limit = page.limit.unwrap_or(self.default_limit);
        if limit > self.max_limit {
            return Err(Error::Input {});
        }
        let end = page.index * Uint128::new((limit + 1) as u128);
        Ok(if end > self.max_items {
            self.max_items
        } else {
            end
        })
    }
}