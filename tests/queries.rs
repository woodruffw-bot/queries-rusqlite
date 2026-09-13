//! Query results, row mapping, and parameter binding.

use queries_rusqlite::{FromRow, queries};
use rusqlite::types::{FromSql, FromSqlResult, ToSqlOutput, ValueRef};
use rusqlite::{Connection, Error, Result, Row, ToSql};

#[derive(Debug, PartialEq, FromRow)]
struct Person {
    id: i64,
    #[column = "display_name"]
    name: String,
    note: Option<String>,
}

#[derive(Debug, PartialEq, FromRow)]
struct Pair(i64, String);

#[derive(Debug, PartialEq, FromRow)]
struct ReorderedPair(#[column = "id"] i64, #[column = "name"] String);

#[derive(Debug, PartialEq, FromRow)]
struct Generic<T> {
    value: T,
}

#[derive(Debug, PartialEq, FromRow)]
struct Keyword {
    r#type: String,
}

#[derive(Debug, PartialEq)]
struct Identifier(i64);

impl ToSql for Identifier {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(self.0.into())
    }
}

impl FromSql for Identifier {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(Self)
    }
}

struct Unbindable;

impl ToSql for Unbindable {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Err(Error::ToSqlConversionFailure(Box::new(
            std::io::Error::other("parameter conversion failed"),
        )))
    }
}

#[derive(Debug)]
struct MissingRowError;

impl FromRow for MissingRowError {
    fn from_row(_: &Row<'_>) -> Result<Self> {
        Err(Error::QueryReturnedNoRows)
    }
}

type MaybeNumber = Option<(i64,)>;
type Numbers = Vec<(i64,)>;
type NoRows = ();

#[queries]
trait TestQueries {
    #[query = "CREATE TABLE person (id INTEGER PRIMARY KEY, name TEXT NOT NULL, note TEXT)"]
    fn create();

    #[query = "INSERT INTO person VALUES (?1, ?2, ?3)"]
    fn insert(id: i64, name: &str, note: Option<&str>) -> NoRows;

    #[query = "SELECT note, name AS display_name, id FROM person WHERE id = ?1"]
    fn person(id: i64) -> Person;

    #[query = "SELECT note, name AS display_name, id FROM person WHERE id = ?1"]
    fn maybe_person(id: i64) -> Option<Person>;

    #[query = "SELECT note, name AS display_name, id FROM person ORDER BY id"]
    fn people() -> Vec<Person>;

    #[query = "SELECT id FROM person ORDER BY id"]
    fn single() -> (i64,);

    #[query = "SELECT id FROM person ORDER BY id"]
    fn optional() -> MaybeNumber;

    #[query = "SELECT id FROM person ORDER BY id"]
    fn numbers() -> Numbers;

    #[query = "SELECT 1"]
    fn mapper_error() -> Option<MissingRowError>;

    #[query = "SELECT ?2, ?1, ?2"]
    fn repeated(first: i64, second: &str) -> (String, i64, String);

    #[query = "SELECT ?1, ?2"]
    fn values(blob: &[u8], nullable: Option<&str>) -> (Vec<u8>, Option<String>);

    #[query = "SELECT ?1"]
    fn custom(value: Identifier) -> (Identifier,);

    #[query = "SELECT ?1"]
    fn binding_error(value: Unbindable) -> (i64,);

    #[query = "SELECT ?1"]
    fn too_few_parameters() -> (i64,);

    #[query = "SELECT 1"]
    fn too_many_parameters(unused: i64) -> (i64,);

    #[query = "SELECT missing FROM nonexistent"]
    fn invalid_sql() -> (i64,);

    #[query = "SELECT missing FROM nonexistent"]
    fn invalid_optional_sql() -> Option<(i64,)>;

    #[query = "SELECT missing FROM nonexistent"]
    fn invalid_vector_sql() -> Vec<(i64,)>;

    #[query = "SELECT ?1"]
    fn vector_binding_error(value: Unbindable) -> Vec<(i64,)>;

    #[query = "SELECT 1; SELECT 2"]
    fn multiple_statements() -> (i64,);

    #[query = "SELECT abs(?1)"]
    fn step_error(value: i64) -> (i64,);

    #[query = "SELECT 1 UNION ALL SELECT abs(?1)"]
    fn second_step_error(value: i64) -> Option<(i64,)>;

    #[query = "SELECT 1 UNION ALL SELECT abs(?1)"]
    fn later_step_error(value: i64) -> Vec<(i64,)>;

    #[query = "SELECT 'text'"]
    fn wrong_type() -> (i64,);

    #[query = "SELECT 1 UNION ALL SELECT 'text'"]
    fn later_wrong_type() -> Vec<(i64,)>;

    #[query = "SELECT 1"]
    fn missing_column() -> (i64, i64);

    #[query = "SELECT 1 AS wrong_name"]
    fn missing_named_column() -> Generic<i64>;

    #[query = "SELECT 1"]
    fn unexpected_rows();

    #[query = "SELECT 12, 'pair'"]
    fn pair() -> Pair;

    #[query = "SELECT 'pair' AS name, 12 AS id"]
    fn reordered_pair() -> ReorderedPair;

    #[query = include_str!("fixtures/value.sql")]
    fn generic(value: Identifier) -> Generic<Identifier>;

    #[query = "SELECT 'keyword' AS type"]
    fn r#type() -> Keyword;

    #[cfg(any())]
    #[query = "SELECT 1"]
    fn disabled_method() -> UndefinedType;
}

#[queries]
#[cfg(any())]
trait DisabledQueries {
    #[query = "SELECT 1"]
    fn value() -> UndefinedType;
}

#[test]
fn row_shapes_and_parameter_binding() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    let q = TestQueries::from_conn(&connection);
    q.create()?;
    let injection = "Robert'); DROP TABLE person; --";
    q.insert(2, injection, Some("note"))?;
    q.insert(1, "Ada", None)?;

    assert_eq!(
        q.person(2)?,
        Person {
            id: 2,
            name: injection.to_owned(),
            note: Some("note".to_owned()),
        }
    );
    assert_eq!(q.maybe_person(1)?, Some(q.person(1)?));
    assert_eq!(q.maybe_person(3)?, None);
    assert_eq!(q.people()?, vec![q.person(1)?, q.person(2)?]);
    assert_eq!(q.numbers()?, vec![(1,), (2,)]);
    assert_eq!(
        q.repeated(42, "bound")?,
        ("bound".to_owned(), 42, "bound".to_owned())
    );
    assert_eq!(q.values(b"\0\xff", None)?, (vec![0, 255], None));
    assert_eq!(q.values(&[], Some(""))?, (Vec::new(), Some(String::new())));
    assert_eq!(q.custom(Identifier(17))?, (Identifier(17),));
    assert_eq!(q.pair()?, Pair(12, "pair".to_owned()));
    assert_eq!(q.reordered_pair()?, ReorderedPair(12, "pair".to_owned()));
    assert_eq!(
        q.generic(Identifier(23))?,
        Generic {
            value: Identifier(23)
        }
    );
    assert_eq!(
        q.r#type()?,
        Keyword {
            r#type: "keyword".to_owned()
        }
    );
    Ok(())
}

#[test]
fn cardinality_is_checked_for_single_and_optional_rows() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    let q = TestQueries::from_conn(&connection);
    q.create()?;
    assert!(matches!(q.single(), Err(Error::QueryReturnedNoRows)));
    assert_eq!(q.optional()?, None);
    assert!(q.numbers()?.is_empty());
    q.insert(1, "one", None)?;
    assert_eq!(q.single()?, (1,));
    assert_eq!(q.optional()?, Some((1,)));
    q.insert(2, "two", None)?;
    assert!(matches!(
        q.single(),
        Err(Error::QueryReturnedMoreThanOneRow)
    ));
    assert!(matches!(
        q.optional(),
        Err(Error::QueryReturnedMoreThanOneRow)
    ));
    Ok(())
}

#[test]
fn errors_keep_their_original_meaning() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    let q = TestQueries::from_conn(&connection);
    assert!(matches!(q.mapper_error(), Err(Error::QueryReturnedNoRows)));
    assert!(matches!(
        q.binding_error(Unbindable),
        Err(Error::ToSqlConversionFailure(_))
    ));
    assert!(matches!(
        q.too_few_parameters(),
        Err(Error::InvalidParameterCount(0, 1))
    ));
    assert!(matches!(
        q.too_many_parameters(1),
        Err(Error::InvalidParameterCount(1, 0))
    ));
    assert!(q.invalid_sql().is_err());
    assert!(q.invalid_optional_sql().is_err());
    assert!(q.invalid_vector_sql().is_err());
    assert!(matches!(
        q.vector_binding_error(Unbindable),
        Err(Error::ToSqlConversionFailure(_))
    ));
    assert!(matches!(
        q.multiple_statements(),
        Err(Error::MultipleStatement)
    ));
    assert!(matches!(
        q.step_error(i64::MIN),
        Err(Error::SqliteFailure(_, _))
    ));
    assert!(matches!(
        q.second_step_error(i64::MIN),
        Err(Error::SqliteFailure(_, _))
    ));
    assert!(matches!(
        q.later_step_error(i64::MIN),
        Err(Error::SqliteFailure(_, _))
    ));
    assert!(matches!(q.wrong_type(), Err(Error::InvalidColumnType(..))));
    assert!(matches!(
        q.later_wrong_type(),
        Err(Error::InvalidColumnType(..))
    ));
    assert!(matches!(
        q.missing_column(),
        Err(Error::InvalidColumnIndex(1))
    ));
    assert!(matches!(
        q.missing_named_column(),
        Err(Error::InvalidColumnName(_))
    ));
    assert!(matches!(
        q.unexpected_rows(),
        Err(Error::ExecuteReturnedResults)
    ));
    Ok(())
}

/// Public declarations checked by the workspace documentation lint.
pub mod visible {
    use queries_rusqlite as renamed;

    /// Shadow the prelude variant to check generated path resolution.
    pub struct Ok;

    /// Public declarations keep their methods accessible to callers.
    #[renamed::queries(crate = renamed)]
    pub trait PublicQueries {
        /// Read a constant.
        #[query = "SELECT 7 AS value"]
        fn value() -> Value;
    }

    /// A decoded integer.
    #[derive(Debug, PartialEq, renamed::FromRow)]
    #[from_row(crate = renamed)]
    pub struct Value {
        /// The selected value.
        pub value: i64,
    }
}

#[test]
fn generated_methods_are_public_and_accept_crate_overrides() -> Result<()> {
    let _ = visible::Ok;
    let connection = Connection::open_in_memory()?;
    let q = visible::PublicQueries::from_conn(&connection);
    assert_eq!(q.value()?, visible::Value { value: 7 });
    Ok(())
}
