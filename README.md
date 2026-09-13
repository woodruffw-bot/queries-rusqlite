# queries-rusqlite

Declare SQLite queries as synchronous Rust methods. This crate follows the
[`queries`](https://docs.rs/queries/0.2.0/queries/) API, using
[`rusqlite`](https://docs.rs/rusqlite/) connections and transactions.

Requires Rust 1.95.0 or later and a system SQLite library. On Debian or Ubuntu,
install `libsqlite3-dev` and `pkg-config`.

Until the crate is published, depend on the Git repository:

```toml
[dependencies]
queries-rusqlite = { git = "https://github.com/woodruffw-bot/queries-rusqlite" }
```

## Queries

```rust
use queries_rusqlite::{FromRow, queries, rusqlite};

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

`#[queries]` replaces the trait with a struct of the same name. Its generated
methods take `&self` and return `rusqlite::Result<DeclaredReturnType>`.
Declarations have no receiver, body, generics, or `async` qualifier.

| Declared return type | Behavior |
| --- | --- |
| `T: FromRow` | Requires exactly one row; zero or multiple rows are errors. |
| `Option<T>` | Returns `None` for zero rows; multiple rows are an error. |
| `Vec<T>` | Collects every row, stopping at the first error. |
| `Query<'_, T>` | Prepares and binds a query for lazy iteration. |
| `()` or no return type | Executes a statement without result rows; discards the affected row count. |

Use `(T,)` to read one column, for example `fn count() -> (i64,);`. Tuples of
one through sixteen elements implement `FromRow`. Return type aliases work too.

Parameters must implement rusqlite's `ToSql`. They bind in declaration order:
the first argument is `?1`, the second is `?2`, and so on. Values are never
interpolated into SQL. SQL may be any Rust expression yielding `&str`, including
`include_str!("query.sql")`. SQL syntax, parameter counts, and column types are
checked at runtime.

## Rows

`#[derive(FromRow)]` reads named struct fields by column name and tuple struct
fields by position. Fields must implement rusqlite's `FromSql`; generic structs
are supported. Override a column name with a field attribute:

```rust
use queries_rusqlite::FromRow;

#[derive(FromRow)]
struct User {
    #[column = "user_id"]
    id: i64,
    name: String,
}
```

For custom decoding, implement
`fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self>` manually.
Returned values own their data and cannot borrow from a SQLite row.

If the dependency is renamed, set `#[queries(crate = renamed)]` and
`#[from_row(crate = renamed)]` on the corresponding declarations.

## Lazy iteration

```rust
use queries_rusqlite::{Query, queries, rusqlite};

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

Creating the handle prepares SQL and binds arguments. Execution starts when
the iterator advances; execution and decoding errors appear as iterator items.
Each call to `iter()` provides a fresh iterator with the same bound values.
The handle borrows the connection, and the iterator borrows the handle. Dropping
the iterator resets the statement; dropping the handle releases it. Neither
undoes writes SQLite has already performed.

## Connections and transactions

| Constructor | Ownership and transaction support |
| --- | --- |
| `Users::from_conn(&connection)` | Borrows a connection for queries. |
| `Users::from_conn_mut(&mut connection)` | Borrows a connection; `begin()` starts a transaction. |
| `Users::from_connection(connection)` | Owns a connection; `begin()` starts a transaction. |
| `Users::from_tx(transaction)` | Owns an existing transaction; exposes `commit()` and `rollback()`. |

`begin()` borrows the wrapper mutably and returns a transaction wrapper with
the same query methods. `commit()` and `rollback()` consume that wrapper.
Unfinished transactions follow rusqlite's drop behavior, which rolls back by
default. Create a transaction with rusqlite and pass it to `from_tx` when you
need to choose its transaction or drop behavior.

## Dependencies and safety

The runtime depends on rusqlite and the accompanying procedural macro crate.
The macro crate uses only `proc-macro2`, `quote`, and `syn`; there are no test
dependencies. Rusqlite's default features are disabled, and SQLite is linked
from the system. The crate re-exports rusqlite so callers can use the same
version without declaring another dependency.

Both crates forbid unsafe Rust, and the macros generate safe Rust. This applies
to this workspace's code: rusqlite and its SQLite bindings use unsafe code for
their FFI implementation.

## Development

Run the same checks as CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

CI tests Rust 1.95.0 and stable. Coverage uses Rust 1.95.0 and
[`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov):

```sh
rustup toolchain install 1.95.0 --profile minimal --component llvm-tools-preview
cargo +1.95.0 install cargo-llvm-cov --locked --version 0.9.1
cargo +1.95.0 llvm-cov --workspace --all-targets --locked \
  --ignore-filename-regex '(/tests/|/src/tests\.rs$)' \
  --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
```

Coverage must reach 100% of lines, functions, and regions in both crates. The
report excludes test files, never production code. Coverage measures execution;
it does not prove the absence of bugs. Documentation examples are tested
separately.

CI also audits the workflows with zizmor 1.30.1 in pedantic mode and fails on
findings. To audit locally with zizmor installed, run the command below. Set
`GH_TOKEN` to include online audits.

```sh
zizmor --persona=pedantic .
```

## License

BSD-3-Clause. The return type dispatch is adapted from
[`queries`](https://github.com/alex/queries-rs); its copyright notice and
license are retained in `LICENSE`.
