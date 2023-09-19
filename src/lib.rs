use cosmwasm_schema::cw_serde;
use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, KeyDeserialize, Map, PrimaryKey};
use std::iter::Take;
use std::marker::PhantomData;

pub type Page<'a, S: PrimaryKey<'a> + KeyDeserialize<Output = S> + Clone + 'static> =
    DefaultPage<50, S>;

/// Used to paginate a query, start is not inclusive
#[cw_serde]
pub struct DefaultPage<const LIMIT: usize, K>
where
    K: KeyDeserialize<Output = K>,
{
    pub start: Option<K>,
    pub qty: Option<usize>,
}

// K stands for the key while O is the output (what the iterator will be using)
impl<'a, K, const LIMIT: usize> DefaultPage<LIMIT, K>
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
    ) -> StdResult<Pagination<D, K::Output>>
    where
        D: Serialize + DeserializeOwned,
        V: Serialize + DeserializeOwned + Clone + 'static,
        A: FnOnce(&K::Output, V) -> D + Copy,
    {
        let mut keys = self.keys(storage, &map);
        let mut data = vec![];
        let mut end = None;

        let mut next = keys.next();
        loop {
            if let Some(key) = next {
                let key = key?;
                let value = map.load(storage, key.clone())?;
                let res = transform(&key, value);
                data.push(res);

                next = keys.next();
                if next.is_none() {
                    end = Some(key);
                }
            } else {
                break;
            }
        }

        let len = data.len();
        Ok(Pagination {
            data,
            next: end,
            qty: len,
        })
    }
}

#[cw_serde]
pub struct Pagination<D, K> {
    pub data: Vec<D>,
    pub next: Option<K>,
    pub qty: usize,
}

#[cfg(test)]
mod test {
    use crate::DefaultPage;
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

        let query: DefaultPage<20, _> = DefaultPage {
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

        let query: DefaultPage<20, _> = DefaultPage {
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

        let query: DefaultPage<20, _> = DefaultPage {
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

        let query: DefaultPage<20, _> = DefaultPage {
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

        let query: DefaultPage<30, _> = DefaultPage {
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
