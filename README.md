# queries-rusqlite

[`queries`](https://docs.rs/queries/0.2.0/queries/)-style query declarations for
[`rusqlite`](https://docs.rs/rusqlite/), with synchronous methods.

Requires Rust 1.95.0 or later.

Install from Git:

```toml
[dependencies]
queries-rusqlite = { git = "https://github.com/woodruffw-bot/queries-rusqlite" }
rusqlite = { version = "0.40.2", default-features = false }
```

## Queries

```rust
use queries_rusqlite::{FromRow, queries};

#[derive(Debug, PartialEq, FromRow)]
struct User {
    id: i64,
    name: String,
}

#[queries]
trait Users {
    #[query = "INSERT INTO users (id, name) VALUES (?1, ?2)"]
    fn insert(id: i64, name: &str);

    #[query = "SELECT id, name FROM users WHERE id = ?1"]
    fn get(id: i64) -> Option<User>;

    #[query = "SELECT id, name FROM users ORDER BY id"]
    fn all() -> Vec<User>;
}

fn main() -> rusqlite::Result<()> {
    let connection = rusqlite::Connection::open_in_memory()?;
    connection.execute_batch("CREATE TABLE users (id INTEGER, name TEXT)")?;

    let users = Users::from_conn(&connection);
    users.insert(1, "Ada")?;
    assert_eq!(users.get(1)?.unwrap().name, "Ada");
    assert_eq!(users.get(2)?, None);
    assert_eq!(users.all()?.len(), 1);
    Ok(())
}
```

`#[queries]` replaces the trait with a struct whose methods take `&self` and
return `rusqlite::Result<T>`. Declare methods without a receiver, body, generics,
or `async` qualifier.

| Declared return type | Behavior |
| --- | --- |
| `T: FromRow` | Exactly one row; otherwise an error. |
| `Option<T>` | Zero or one row; multiple rows are an error. |
| `Vec<T>` | All rows, stopping at the first error. |
| `Query<'_, T>` | Prepares and binds a query for lazy iteration. |
| `()` or no return type | Executes a statement without result rows; discards the affected row count. |

Use `(T,)` to read one column, for example `fn count() -> (i64,);`. Tuples of
one through sixteen elements and return type aliases are supported.

Arguments must implement `rusqlite::ToSql` and bind in declaration order to
`?1`, `?2`, and so on. SQL can be any expression yielding `&str`, including
`include_str!("query.sql")`. SQL and column types are checked at runtime.

## Rows

`#[derive(FromRow)]` reads named fields by column name and tuple fields by
position. Use `#[column = "user_id"]` on a field to override its column name.
Fields must implement `rusqlite::types::FromSql`; generic structs are supported.

For custom decoding, implement
`fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self>` manually.
Returned values own their data and cannot borrow from a SQLite row.

If `queries-rusqlite` is renamed, set `#[queries(crate = renamed)]` and
`#[from_row(crate = renamed)]` on the corresponding declarations.

## Lazy iteration

```rust
use queries_rusqlite::{Query, queries};

#[queries]
trait Numbers {
    #[query = "SELECT ?1 UNION ALL SELECT ?1 + 1"]
    fn starting_at(value: i64) -> Query<'_, (i64,)>;
}

fn main() -> rusqlite::Result<()> {
    let connection = rusqlite::Connection::open_in_memory()?;
    let numbers = Numbers::from_conn(&connection);
    let mut query = numbers.starting_at(7)?;
    for row in query.iter() {
        let (number,) = row?;
        println!("{number}");
    }
    Ok(())
}
```

The query prepares SQL and binds arguments immediately, but executes only when
the iterator advances. A new iterator starts from the beginning with the same
bound values. Errors appear as iterator items. The query borrows its connection,
and the iterator borrows the query. Dropping either does not undo writes.

## Connections and transactions

| Constructor | Ownership and transaction support |
| --- | --- |
| `Users::from_conn(&connection)` | Borrows a connection for queries. |
| `Users::from_conn_mut(&mut connection)` | Borrows a connection; `begin()` starts a transaction. |
| `Users::from_connection(connection)` | Owns a connection; `begin()` starts a transaction. |
| `Users::from_tx(transaction)` | Owns an existing transaction; exposes `commit()` and `rollback()`. |

`begin()` takes `&mut self` and returns a transaction with the same query
methods. `commit()` and `rollback()` consume it. Dropping an unfinished
transaction rolls it back by default. To configure transaction or drop behavior,
create a rusqlite transaction and pass it to `from_tx`.

## License

BSD-3-Clause. The return type dispatch is adapted from
[`queries`](https://github.com/alex/queries-rs); its copyright notice and
license are retained in `LICENSE`.
