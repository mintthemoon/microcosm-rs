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
    pub default: u32,
    pub max: u32,
}

impl PageLimits {
    pub fn start_index(&self, page: PageQuery) -> Res<Uint128> {
        let limit = page.limit.unwrap_or(self.default);
        if limit > self.max {
            return Err(Error::Input {});
        }
        Ok(page.index * Uint128::new(limit as u128))
    }

    pub fn end_index(&self, page: PageQuery) -> Res<Uint128> {
        let limit = page.limit.unwrap_or(self.default);
        if limit > self.max {
            return Err(Error::Input {});
        }
        Ok(page.index * Uint128::new((limit + 1) as u128))
    }
}