use crate::Pagination;
use cosmwasm_schema::cw_serde;
use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, KeyDeserialize, Map, PrimaryKey};
use std::iter::Take;
use std::marker::PhantomData;

pub type DefaultQuery<'a, S> = Query<50, S>;

#[cw_serde]
pub struct Query<const LIMIT: usize, K> {
    pub start: Option<K>,
    pub qty: Option<usize>,
}

impl<'a, const LIMIT: usize, K> Query<LIMIT, K>
where
    K: PrimaryKey<'a> + KeyDeserialize<Output = K> + Clone + 'static,
{
    /// Get an iterator of map keys
    pub fn keys<V>(
        self,
        storage: &'a dyn Storage,
        map: &Map<'a, K, V>,
    ) -> Take<Box<dyn Iterator<Item = StdResult<K::Output>> + 'a>>
    where
        V: Serialize + DeserializeOwned + Clone + 'static,
    {
        map.keys(
            storage,
            self.start.map(|s| Bound::Exclusive((s, PhantomData))),
            None,
            Order::Ascending,
        )
        .take(self.qty.unwrap_or(LIMIT))
    }

    pub fn into_pagination<D, V, A>(
        self,
        storage: &'a dyn Storage,
        map: &Map<'a, K, V>,
        transform: A,
    ) -> StdResult<Pagination<D, K>>
    where
        D: Serialize + DeserializeOwned,
        V: Serialize + DeserializeOwned + Clone + 'static,
        A: FnOnce(K, V) -> D + Copy,
    {
        let mut range = map
            .range(
                storage,
                self.start.map(|s| Bound::Exclusive((s, PhantomData))),
                None,
                Order::Ascending,
            )
            .take(self.qty.unwrap_or(LIMIT));
        let mut data = vec![];
        let mut end = None;

        let mut next = range.next();

        while let Some(key) = next {
            let (key, value) = key?;

            next = range.next();
            if next.is_none() {
                end = Some(key.clone());
            }

            let res = transform(key, value);
            data.push(res);
        }

        let len = data.len();
        Ok(Pagination {
            data,
            next: end,
            qty: len,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::Query;
    use cosmwasm_std::testing::mock_dependencies;
    use cw_storage_plus::Map;

    #[test]
    fn pagination_iterator() {
        let mut deps = mock_dependencies();
        let test_map: Map<u8, String> = Map::new("test_map");

        for i in 0..100 {
            test_map
                .save(deps.as_mut().storage, i, &format!("string-{}", i))
                .unwrap();
        }

        assert_eq!(
            test_map.load(deps.as_ref().storage, 2).unwrap(),
            "string-2".to_string()
        );

        let query: Query<20, _> = Query {
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

        let query: Query<20, _> = Query {
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

        let query: Query<20, _> = Query {
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
        let test_map: Map<String, u8> = Map::new("test_map");

        for i in 0..100 {
            test_map
                .save(deps.as_mut().storage, format!("string-{:0>3}", i), &i)
                .unwrap();
        }

        let query: Query<20, _> = Query {
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

        let query: Query<30, _> = Query {
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
