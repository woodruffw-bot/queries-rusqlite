//! Declare SQLite queries as synchronous Rust methods.
//!
//! Add both `queries-rusqlite` and `rusqlite` to your dependencies. Generated
//! code references `rusqlite` directly, so keep that dependency name.
//!
//! ```
//! use queries_rusqlite::{queries, FromRow};
//!
//! #[derive(Debug, PartialEq, FromRow)]
//! struct User {
//!     id: i64,
//!     name: String,
//! }
//!
//! #[queries]
//! trait Users {
//!     #[query = "SELECT id, name FROM users WHERE id = ?1"]
//!     fn get(id: i64) -> Option<User>;
//!
//!     #[query = "INSERT INTO users (id, name) VALUES (?1, ?2)"]
//!     fn insert(id: i64, name: &str);
//! }
//!
//! # fn main() -> rusqlite::Result<()> {
//! let connection = rusqlite::Connection::open_in_memory()?;
//! connection.execute_batch("CREATE TABLE users (id INTEGER, name TEXT)")?;
//! let users = Users::from_conn(&connection);
//! users.insert(1, "Ada")?;
//! assert_eq!(users.get(1)?.unwrap().name, "Ada");
//! assert_eq!(users.get(2)?, None);
//! # Ok(())
//! # }
//! ```
//!
//! Methods bind arguments in declaration order and return [`rusqlite::Result`].
//! A return type implementing [`FromRow`] requires exactly one row;
//! `Option<T>` accepts zero or one, and `Vec<T>` collects all rows. [`Query`]
//! provides lazy iteration. A method returning `()` executes a statement that
//! does not return rows. SQL and column types are checked when the method runs.
//!
//! This crate follows the API of [queries](https://docs.rs/queries/0.2.0/queries/),
//! using rusqlite connections instead of asynchronous connection pools.

use std::marker::PhantomData;

pub use queries_rusqlite_macros::{FromRow, queries};

/// Decode an owned value from a SQLite row.
///
/// Derive this trait for structs, or implement it for custom decoding. Named
/// fields use column names; tuple fields use column positions. Tuples of one
/// through sixteen elements implement this trait when every element implements
/// [`rusqlite::types::FromSql`]. Use `(T,)` to read a single column.
///
/// Returned values cannot borrow from the row: rusqlite reuses its storage when
/// it advances to the next row.
pub trait FromRow: Sized {
    /// Decode a row, preserving any column lookup or conversion error.
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self>;
}

macro_rules! tuple_from_row {
    ($($name:ident:$index:tt),+) => {
        impl<$($name: rusqlite::types::FromSql),+> FromRow for ($($name,)+) {
            fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
                Ok(($(row.get::<_, $name>($index)?,)+))
            }
        }
    };
}

tuple_from_row!(A:0);
tuple_from_row!(A:0, B:1);
tuple_from_row!(A:0, B:1, C:2);
tuple_from_row!(A:0, B:1, C:2, D:3);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14);
tuple_from_row!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, P:15);

/// A prepared query that decodes rows during iteration.
///
/// Declare a query method returning `Query<'_, T>` to obtain this handle. The
/// method prepares SQL and binds its arguments, but does not execute the query.
/// Each call to [`iter`](Self::iter) starts a new execution with the same bound
/// values. Execution and decoding errors appear as iterator items.
///
/// ```
/// use queries_rusqlite::{queries, Query};
/// #[queries]
/// trait Numbers {
///     #[query = "SELECT ?1 UNION ALL SELECT ?1 + 1"]
///     fn starting_at(value: i64) -> Query<'_, (i64,)>;
/// }
/// # fn main() -> rusqlite::Result<()> {
/// let connection = rusqlite::Connection::open_in_memory()?;
/// let numbers = Numbers::from_conn(&connection);
/// let mut query = numbers.starting_at(7)?;
/// for row in query.iter() {
///     let (number,) = row?;
///     assert!(number == 7 || number == 8);
/// }
/// # Ok(())
/// # }
/// ```
///
/// The handle borrows its connection. Its iterator borrows the handle, so a
/// transaction cannot be committed while either is in use. Dropping an iterator
/// resets the statement; it does not undo writes already performed by SQLite.
///
/// A query cannot outlive the wrapper it borrows:
///
/// ```compile_fail
/// use queries_rusqlite::{queries, Query};
/// #[queries]
/// trait Numbers {
///     #[query = "SELECT 1"]
///     fn all() -> Query<'_, (i64,)>;
/// }
/// let connection = rusqlite::Connection::open_in_memory().unwrap();
/// let mut query = {
///     let numbers = Numbers::from_conn(&connection);
///     numbers.all().unwrap()
/// };
/// query.iter().next();
/// ```
///
/// A live query also prevents committing its transaction:
///
/// ```compile_fail
/// use queries_rusqlite::{queries, Query};
/// #[queries]
/// trait Numbers {
///     #[query = "SELECT 1"]
///     fn all() -> Query<'_, (i64,)>;
/// }
/// let mut connection = rusqlite::Connection::open_in_memory().unwrap();
/// let numbers = Numbers::from_tx(connection.transaction().unwrap());
/// let mut query = numbers.all().unwrap();
/// numbers.commit().unwrap();
/// query.iter().next();
/// ```
pub struct Query<'conn, T> {
    statement: rusqlite::Statement<'conn>,
    row_type: PhantomData<fn() -> T>,
}

impl<T: FromRow> Query<'_, T> {
    /// Iterate from the beginning using the bound parameters.
    ///
    /// Execution starts when the iterator advances. Each item is a decoded row
    /// or an execution or decoding error. Dropping the iterator resets the
    /// statement without undoing writes.
    pub fn iter(&mut self) -> impl Iterator<Item = rusqlite::Result<T>> + '_ {
        self.statement.raw_query().mapped(T::from_row)
    }
}

/// Implementation details used by the procedural macro.
#[doc(hidden)]
pub mod __private {
    use super::{FromRow, PhantomData, Query};
    use rusqlite::{Connection, OptionalExtension, ToSql, Transaction};

    const ONE_ROW: u8 = 0;
    const OPTIONAL_ROW: u8 = 1;
    const ALL_ROWS: u8 = 2;
    const LAZY_ROWS: u8 = 3;
    const EXECUTE: u8 = 4;

    /// Access the connection held by a generated wrapper.
    pub trait ConnectionSource {
        /// Borrow the underlying connection.
        fn connection(&self) -> &Connection;
    }

    impl ConnectionSource for Connection {
        fn connection(&self) -> &Connection {
            self
        }
    }

    impl ConnectionSource for &Connection {
        fn connection(&self) -> &Connection {
            self
        }
    }

    impl ConnectionSource for &mut Connection {
        fn connection(&self) -> &Connection {
            self
        }
    }

    impl ConnectionSource for Transaction<'_> {
        fn connection(&self) -> &Connection {
            self
        }
    }

    // Inherent constants take precedence over the trait's fallback. Resolving
    // this in Rust (rather than matching type names in the macro) permits aliases.
    /// Select single-row decoding when no specialized category applies.
    pub trait SingleRowFallback {
        /// The single-row category.
        const VALUE: u8 = ONE_ROW;
    }

    /// Select result decoding by type, including type aliases.
    pub struct FromRowsCategory<T>(PhantomData<T>);
    impl<T> SingleRowFallback for FromRowsCategory<T> {}
    impl<T> FromRowsCategory<Option<T>> {
        /// Zero or one row.
        pub const VALUE: u8 = OPTIONAL_ROW;
    }
    impl<T> FromRowsCategory<Vec<T>> {
        /// All rows.
        pub const VALUE: u8 = ALL_ROWS;
    }
    impl<T> FromRowsCategory<Query<'_, T>> {
        /// Lazy iteration.
        pub const VALUE: u8 = LAZY_ROWS;
    }
    impl FromRowsCategory<()> {
        /// Execution without result rows.
        pub const VALUE: u8 = EXECUTE;
    }

    /// Execute SQL and decode its result using the selected category.
    pub trait FromRows<'conn, const CATEGORY: u8>: Sized {
        /// Prepare SQL, bind parameters, and produce the declared result.
        fn from_rows(
            connection: &'conn Connection,
            sql: &str,
            params: &[&dyn ToSql],
        ) -> rusqlite::Result<Self>;
    }

    impl<T: FromRow> FromRows<'_, ONE_ROW> for T {
        fn from_rows(
            connection: &Connection,
            sql: &str,
            params: &[&dyn ToSql],
        ) -> rusqlite::Result<Self> {
            connection.prepare(sql)?.query_one(params, T::from_row)
        }
    }

    impl<T: FromRow> FromRows<'_, OPTIONAL_ROW> for Option<T> {
        fn from_rows(
            connection: &Connection,
            sql: &str,
            params: &[&dyn ToSql],
        ) -> rusqlite::Result<Self> {
            // Keep a decoder's QueryReturnedNoRows error inside the result so
            // OptionalExtension only handles the absence of a database row.
            connection
                .prepare(sql)?
                .query_one(params, |row| Ok(T::from_row(row)))
                .optional()?
                .transpose()
        }
    }

    impl<T: FromRow> FromRows<'_, ALL_ROWS> for Vec<T> {
        fn from_rows(
            connection: &Connection,
            sql: &str,
            params: &[&dyn ToSql],
        ) -> rusqlite::Result<Self> {
            connection
                .prepare(sql)?
                .query_map(params, T::from_row)?
                .collect()
        }
    }

    impl<'conn, T: FromRow> FromRows<'conn, LAZY_ROWS> for Query<'conn, T> {
        fn from_rows(
            connection: &'conn Connection,
            sql: &str,
            params: &[&dyn ToSql],
        ) -> rusqlite::Result<Self> {
            let mut statement = connection.prepare(sql)?;
            let expected = statement.parameter_count();
            if params.len() != expected {
                return Err(rusqlite::Error::InvalidParameterCount(
                    params.len(),
                    expected,
                ));
            }
            for (index, &parameter) in params.iter().enumerate() {
                statement.raw_bind_parameter(index + 1, parameter)?;
            }
            Ok(Self {
                statement,
                row_type: PhantomData,
            })
        }
    }

    impl FromRows<'_, EXECUTE> for () {
        fn from_rows(
            connection: &Connection,
            sql: &str,
            params: &[&dyn ToSql],
        ) -> rusqlite::Result<Self> {
            connection.execute(sql, params).map(|_| ())
        }
    }
}
