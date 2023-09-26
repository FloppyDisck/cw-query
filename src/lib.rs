pub mod prefix;
pub mod query;

pub use prefix::*;
pub use query::*;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;
use std::iter::Take;

#[cw_serde]
pub struct NextPage<D, K> {
    pub data: Vec<D>,
    pub next: Option<K>,
    pub qty: usize,
}

pub trait MapQuery<'a, K, OUTPUT, PREFIX> {
    fn keys<V>(
        self,
        storage: &'a dyn Storage,
        map: &Map<'a, K, V>,
    ) -> Take<Box<dyn Iterator<Item = StdResult<OUTPUT>> + 'a>>;
}
