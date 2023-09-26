use crate::{KeysQuery, NextPage, PaginatedQuery};
use cosmwasm_schema::cw_serde;
use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, KeyDeserialize, Map, PrimaryKey};
use std::iter::Take;
use std::marker::PhantomData;

pub type DefaultPrefixPage<'a, Key, Prefix, Suffix> = PrefixPage<'a, 50, Key, Prefix, Suffix>;
#[cw_serde]
pub struct PrefixPage<'a, const LIMIT: usize, Key, Prefix, Suffix>
where
    Key: PrimaryKey<'a, Prefix = Prefix, Suffix = Suffix>,
    Suffix: PrimaryKey<'a> + KeyDeserialize + Serialize + DeserializeOwned + Clone,
    Prefix: Serialize,
{
    pub prefix: Key::Prefix,
    pub start: Option<Key::Suffix>,
    pub qty: Option<usize>,
}

impl<'a, const LIMIT: usize, Key, Prefix, Suffix, SO, Value, Data>
    PaginatedQuery<'a, Key, Value, Data> for PrefixPage<'a, LIMIT, Key, Prefix, Suffix>
where
    Key: PrimaryKey<'a, Prefix = Prefix, Suffix = Suffix>
        + KeyDeserialize<Output = Key>
        + Clone
        + 'static,
    Prefix: Serialize + DeserializeOwned,
    Suffix: PrimaryKey<'a> + KeyDeserialize<Output = SO> + Serialize + DeserializeOwned + Clone,
    SO: Clone + 'static,
    Value: Serialize + DeserializeOwned + Clone + 'static,
    Data: Serialize + DeserializeOwned,
{
    type POutput = NextPage<Data, Suffix::Output>;
    type FuncKey = Suffix::Output;

    fn into_pagination<Function>(
        self,
        storage: &'a dyn Storage,
        map: &Map<'a, Key, Value>,
        transform: Function,
    ) -> StdResult<Self::POutput>
    where
        Function: FnOnce(Self::FuncKey, Value) -> Data + Copy,
    {
        let mut keys = map
            .prefix(self.prefix)
            .range(
                storage,
                self.start.map(|s| Bound::Exclusive((s, PhantomData))),
                None,
                Order::Ascending,
            )
            .take(self.qty.unwrap_or(LIMIT));
        let mut data = vec![];
        let mut end = None;

        let mut next = keys.next();
        while let Some(key) = next {
            let (key, value) = key?;
            let res = transform(key.clone(), value);
            data.push(res);

            next = keys.next();
            if next.is_none() {
                end = Some(key);
            }
        }

        let len = data.len();
        Ok(NextPage {
            data,
            next: end,
            qty: len,
        })
    }
}

impl<'a, const LIMIT: usize, Key, Prefix, Suffix, SO, Value> KeysQuery<'a, Key, Value>
    for PrefixPage<'a, LIMIT, Key, Prefix, Suffix>
where
    Key: PrimaryKey<'a, Prefix = Prefix, Suffix = Suffix>
        + KeyDeserialize<Output = Key>
        + Clone
        + 'static,
    Prefix: Serialize + DeserializeOwned,
    Suffix: PrimaryKey<'a> + KeyDeserialize<Output = SO> + Serialize + DeserializeOwned + Clone,
    SO: 'static,
    Value: Serialize + DeserializeOwned + Clone + 'static,
{
    type KOutput = Suffix::Output;
    fn keys(
        self,
        storage: &'a dyn Storage,
        map: &Map<'a, Key, Value>,
    ) -> Take<Box<dyn Iterator<Item = StdResult<Self::KOutput>> + 'a>> {
        map.prefix(self.prefix)
            .keys(
                storage,
                self.start.map(|s| Bound::Exclusive((s, PhantomData))),
                None,
                Order::Ascending,
            )
            .take(self.qty.unwrap_or(LIMIT))
    }
}

#[cfg(test)]
mod test {
    use crate::{KeysQuery, PaginatedQuery, PrefixPage};
    use cosmwasm_std::testing::mock_dependencies;
    use cw_storage_plus::Map;

    #[test]
    fn pagination_iterator() {
        let mut deps = mock_dependencies();
        let test_map: Map<(u8, u8), String> = Map::new("test_map");

        for i in 0..100 {
            test_map
                .save(deps.as_mut().storage, (1, i), &format!("string-{}", i))
                .unwrap();
        }

        assert_eq!(
            test_map.load(deps.as_ref().storage, (1, 2)).unwrap(),
            "string-2".to_string()
        );

        let query: PrefixPage<20, _, _, _> = PrefixPage {
            prefix: 1,
            start: None,
            qty: None,
        };

        let mut keys = query.keys(deps.as_ref().storage, &test_map);
        for i in 0..26 {
            let key_op = keys.next();

            if i > 19 {
                assert!(key_op.is_none())
            } else {
                assert!(key_op.is_some());
                let key = key_op.unwrap().unwrap();
                assert_eq!(key, i);
            }
        }

        let query: PrefixPage<20, _, _, _> = PrefixPage {
            prefix: 1,
            start: None,
            qty: Some(5),
        };

        let mut keys = query.keys(deps.as_ref().storage, &test_map);
        for i in 0..10 {
            let key_op = keys.next();

            if i > 4 {
                assert!(key_op.is_none())
            } else {
                assert!(key_op.is_some());
                let key = key_op.unwrap().unwrap();
                assert_eq!(key, i);
            }
        }

        let query: PrefixPage<20, _, _, _> = PrefixPage {
            prefix: 1,
            start: Some(5),
            qty: Some(5),
        };

        let mut keys = query.keys(deps.as_ref().storage, &test_map);
        for i in 6..17 {
            let key_op = keys.next();

            if i > 10 {
                assert!(key_op.is_none())
            } else {
                assert!(key_op.is_some());
                let key = key_op.unwrap().unwrap();
                assert_eq!(key, i);
            }
        }
    }

    #[test]
    fn into_pagination() {
        let mut deps = mock_dependencies();
        let test_map: Map<(u8, String), u8> = Map::new("test_map");

        for i in 0..100 {
            test_map
                .save(deps.as_mut().storage, (1, format!("string-{:0>3}", i)), &i)
                .unwrap();
        }

        let query: PrefixPage<20, _, _, _> = PrefixPage {
            prefix: 1,
            start: None,
            qty: None,
        };

        let res = query
            .into_pagination(deps.as_ref().storage, &test_map, |k, _v| {
                format!("new-{}", k)
            })
            .unwrap();

        assert_eq!(res.qty, 20);
        assert_eq!(res.qty, res.data.len());
        assert!(res.next.is_some());

        assert_eq!(res.next, Some("string-019".to_string()));
        assert_eq!(res.data.get(0).unwrap(), "new-string-000");
        assert_eq!(res.data.get(19).unwrap(), "new-string-019");

        let query: PrefixPage<30, _, _, _> = PrefixPage {
            prefix: 1,
            start: res.next,
            qty: Some(15),
        };

        let res = query
            .into_pagination(deps.as_ref().storage, &test_map, |k, _| k.clone())
            .unwrap();

        assert_eq!(res.qty, 15);
        assert_eq!(res.next, Some("string-034".to_string()));
        assert_eq!(res.data.get(0).unwrap(), "string-020");
    }
}
