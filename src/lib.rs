pub mod prefix;
pub mod query;

pub use prefix::*;
pub use query::*;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{KeyDeserialize, Map};
use std::iter::Take;

#[cw_serde]
pub struct NextPage<D, K> {
    pub data: Vec<D>,
    pub next: Option<K>,
    pub qty: usize,
}

pub trait PaginatedQuery<'a, Key, Value, Data> {
    /// Expected pagination output
    type POutput;

    /// Expected key param in the function
    type FuncKey;
    fn into_pagination<Function>(
        self,
        storage: &'a dyn Storage,
        map: &Map<'a, Key, Value>,
        transform: Function,
    ) -> StdResult<Self::POutput>
    where
        Function: FnOnce(Self::FuncKey, Value) -> Data + Copy;
}

pub trait KeysQuery<'a, Key, Value>
where
    Key: KeyDeserialize<Output = Key> + Clone,
{
    type KOutput;
    fn keys(
        self,
        storage: &'a dyn Storage,
        map: &Map<'a, Key, Value>,
    ) -> Take<Box<dyn Iterator<Item = StdResult<Self::KOutput>> + 'a>>;
}
