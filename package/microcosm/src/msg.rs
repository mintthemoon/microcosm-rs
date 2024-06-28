use crate::{
    schema::cw_serde,
    std::Uint128,
    page::{PageMsg, PageQuery},
};

#[cw_serde]
pub struct MetadataMsg {
    pub page: Option<PageMsg>,
}

impl MetadataMsg {
    fn add_page(&mut self, index: Uint128, end: Uint128) {
        self.page = Some(PageMsg{ index, end });
    }
}

impl Default for MetadataMsg {
    fn default() -> Self {
        MetadataMsg {
            page: None,
        }
    }
}

#[cw_serde]
pub struct MetaMsg<T> {
    meta: MetadataMsg,
    data: T,
}

impl <T> MetaMsg<T> {
    pub fn new(data: T) -> Self {
        MetaMsg {
            meta: MetadataMsg::default(),
            data,
        }
    }

    pub fn add_metadata_page(&mut self, index: Uint128, end: Uint128) {
        self.meta.add_page(index, end);
    }
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
