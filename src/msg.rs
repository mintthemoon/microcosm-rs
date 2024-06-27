use crate::{
    schema::cw_serde,
    page::{PageMsg, PageQuery},
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

#[cw_serde]
pub struct MetadataQuery {
    pub page: Option<PageQuery>,
}

#[cw_serde]
pub struct MetaQuery<T> {
    meta: Option<MetadataQuery>,
    query: T,
}
