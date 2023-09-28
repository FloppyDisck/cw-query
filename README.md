# cw-query
This is a work in progress basic wrapper for `cw-storage-plus` maps. If your have any proposed improvements or have made a fix yourself, please write an Issue or PR for it.

# Implementing a Page
Let's assume we have a map storage that maps user's addresses to their token amounts in a CW20
```rust
pub const BALANCE: Map<'static, Addr, u128> = Map::new("balances");
```

Now all we need is a query function with this package as the argument and return statement
```rust
// Return 20 items max, if the received query asks for more, it will get overrided to the limit
pub fn query_balance(deps: &Deps, page: Page<20, Addr>) -> Result<NextPage<u128, Addr>, Error> {
    page.into_pagination(deps.storage, &BALANCE, |key, value| { value })
}
```

# Implementing a Prefixed Page
This type of page allows you to query maps with a given prefix

Expanding on the previous example, here we have addresses with a timestamp of when they spend their tokens
```rust
pub const SPEND_HISTORY: Map<'static, (Addr, u64), u128> = Map::new("spend_history");
```

Now lets define two functions, one that queries all histories and the other queries a user's history

```rust
pub fn query_all_history(deps: &Deps, page: Page<40, (Addr, u64)>) -> Result<NextPage<u128, (Addr, u64)>, Error> {
    page.into_pagination(deps.storage, &SPEND_HISTORY, |key, value| { value })
}

pub fn query_user_history(deps: &Deps, page: PrefixPage<40, (Addr, u64), Addr, u64>) -> Result<NextPage<u128, Addr>, Error> {
    page.into_pagination(deps.storage, &SPEND_HISTORY, |key, value| { value })
}
```