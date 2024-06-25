use cosmwasm_std::Uint64;

use crate::schema::cw_serde;

#[cw_serde]
pub struct PageMsg<T> {
    pub page: Uint64,
    pub end: Uint64,
    pub items: Vec<T>,
}