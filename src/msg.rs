use crate::{
    schema::cw_serde,
    page::PageMsg,
};

#[cw_serde]
pub struct MetadataMsg {
    pub page: Option<PageMsg>,
}

#[cw_serde]
pub struct MetaMsg<T> {
    meta: MetadataMsg,
    data: T,
}