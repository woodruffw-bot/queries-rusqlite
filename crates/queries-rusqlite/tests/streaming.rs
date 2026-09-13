//! Lazy execution and iteration over prepared queries.

use queries_rusqlite::{Query, queries};
use rusqlite::types::ToSqlOutput;
use rusqlite::{Connection, Error, Result, ToSql};

type NumberQuery<'conn> = Query<'conn, (i64,)>;

struct Unbindable;

impl ToSql for Unbindable {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Err(Error::ToSqlConversionFailure(Box::new(
            std::io::Error::other("parameter conversion failed"),
        )))
    }
}

#[queries]
trait Streams {
    #[query = "SELECT ?1 UNION ALL SELECT ?1 + 1 UNION ALL SELECT ?1 + 2"]
    fn numbers(start: i64) -> NumberQuery<'_>;

    #[query = "SELECT 1 WHERE 0"]
    fn empty() -> Query<'_, (i64,)>;

    #[query = "INSERT INTO event VALUES (?1) RETURNING value"]
    fn insert(value: i64) -> Query<'_, (i64,)>;

    #[query = "SELECT ?1"]
    fn text(value: &str) -> Query<'_, (String,)>;

    #[query = "SELECT 1 UNION ALL SELECT 'wrong type' UNION ALL SELECT 3"]
    fn conversion_error() -> Query<'_, (i64,)>;

    #[query = "SELECT 1 UNION ALL SELECT abs(?1)"]
    fn step_error(value: i64) -> Query<'_, (i64,)>;

    #[query = "SELECT abs(?1)"]
    fn first_step_error(value: i64) -> Query<'_, (i64,)>;

    #[query = "SELECT ?1"]
    fn binding_error() -> Query<'_, (i64,)>;

    #[query = "SELECT 1"]
    fn too_many_parameters(unused: i64) -> Query<'_, (i64,)>;

    #[query = "SELECT ?1"]
    fn conversion_binding_error(value: Unbindable) -> Query<'_, (i64,)>;

    #[query = "SELECT missing FROM missing_table"]
    fn invalid_sql() -> Query<'_, (i64,)>;
}

#[test]
fn iterators_read_rows_and_can_be_restarted() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    let q = Streams::from_conn(&connection);
    let mut query = q.numbers(4)?;
    let mut rows = query.iter();
    assert_eq!(rows.next().transpose()?, Some((4,)));
    drop(rows);
    assert_eq!(
        query.iter().collect::<Result<Vec<_>>>()?,
        vec![(4,), (5,), (6,)]
    );
    assert_eq!(
        query.iter().collect::<Result<Vec<_>>>()?,
        vec![(4,), (5,), (6,)]
    );

    let mut empty = q.empty()?;
    let mut rows = empty.iter();
    assert!(rows.next().is_none());
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn statements_execute_only_when_iteration_advances() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    connection.execute("CREATE TABLE event (value INTEGER)", [])?;
    let count =
        || connection.query_row("SELECT count(*) FROM event", [], |row| row.get::<_, i64>(0));
    let q = Streams::from_conn(&connection);
    let mut query = q.insert(7)?;
    assert_eq!(count()?, 0);
    let mut rows = query.iter();
    assert_eq!(count()?, 0);
    assert_eq!(rows.next().transpose()?, Some((7,)));
    assert_eq!(count()?, 1);
    assert!(rows.next().is_none());
    assert!(rows.next().is_none());
    drop(rows);
    assert_eq!(query.iter().collect::<Result<Vec<_>>>()?, vec![(7,)]);
    assert_eq!(count()?, 2);
    Ok(())
}

#[test]
fn bound_values_outlive_their_arguments() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    let q = Streams::from_conn(&connection);
    let mut query = {
        let text = String::from("owned by SQLite after binding");
        q.text(&text)?
    };
    assert_eq!(
        query.iter().collect::<Result<Vec<_>>>()?,
        vec![(String::from("owned by SQLite after binding"),)]
    );
    Ok(())
}

#[test]
fn mapping_errors_do_not_hide_later_rows() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    let q = Streams::from_conn(&connection);
    let mut query = q.conversion_error()?;
    let mut rows = query.iter();
    assert_eq!(rows.next().transpose()?, Some((1,)));
    assert!(matches!(
        rows.next(),
        Some(Err(Error::InvalidColumnType(..)))
    ));
    assert_eq!(rows.next().transpose()?, Some((3,)));
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn query_errors_are_reported_at_the_correct_stage() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    let q = Streams::from_conn(&connection);
    assert!(matches!(
        q.binding_error(),
        Err(Error::InvalidParameterCount(0, 1))
    ));
    assert!(matches!(
        q.too_many_parameters(1),
        Err(Error::InvalidParameterCount(1, 0))
    ));
    assert!(matches!(
        q.conversion_binding_error(Unbindable),
        Err(Error::ToSqlConversionFailure(_))
    ));
    assert!(q.invalid_sql().is_err());

    let mut query = q.first_step_error(i64::MIN)?;
    let mut rows = query.iter();
    assert!(matches!(rows.next(), Some(Err(Error::SqliteFailure(_, _)))));
    assert!(rows.next().is_none());
    drop(rows);

    let mut query = q.step_error(i64::MIN)?;
    let mut rows = query.iter();
    assert_eq!(rows.next().transpose()?, Some((1,)));
    assert!(matches!(rows.next(), Some(Err(Error::SqliteFailure(_, _)))));
    assert!(rows.next().is_none());
    Ok(())
}
