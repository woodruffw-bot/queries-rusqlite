//! Connection ownership and transaction completion.

use queries_rusqlite::queries;
use rusqlite::{Connection, DropBehavior, Error, Result};

#[queries]
trait Store {
    #[query = "CREATE TABLE item (value INTEGER NOT NULL)"]
    fn create();

    #[query = "INSERT INTO item VALUES (?1)"]
    fn insert(value: i64);

    #[query = "SELECT value FROM item ORDER BY value"]
    fn items() -> Vec<(i64,)>;

    #[query = "ROLLBACK"]
    fn end_transaction();
}

#[test]
fn owned_connection_can_commit_rollback_and_drop_transactions() -> Result<()> {
    let mut store = Store::from_connection(Connection::open_in_memory()?);
    store.create()?;
    let transaction = store.begin()?;
    transaction.insert(1)?;
    assert_eq!(transaction.items()?, vec![(1,)]);
    transaction.commit()?;
    assert_eq!(store.items()?, vec![(1,)]);

    let transaction = store.begin()?;
    transaction.insert(2)?;
    transaction.rollback()?;
    assert_eq!(store.items()?, vec![(1,)]);

    {
        let transaction = store.begin()?;
        transaction.insert(3)?;
    }
    assert_eq!(store.items()?, vec![(1,)]);
    Ok(())
}

#[test]
fn mutable_connection_and_existing_transactions_are_supported() -> Result<()> {
    let mut connection = Connection::open_in_memory()?;
    {
        let mut store = Store::from_conn_mut(&mut connection);
        store.create()?;
        store.insert(1)?;
        let transaction = store.begin()?;
        transaction.insert(2)?;
        transaction.commit()?;
    }

    let transaction = connection.transaction()?;
    let store = Store::from_tx(transaction);
    store.insert(3)?;
    store.rollback()?;
    assert_eq!(Store::from_conn(&connection).items()?, vec![(1,), (2,)]);

    let transaction = connection.transaction()?;
    {
        let borrowed = Store::from_conn(&transaction);
        borrowed.insert(4)?;
    }
    transaction.commit()?;
    assert_eq!(
        Store::from_conn(&connection).items()?,
        vec![(1,), (2,), (4,)]
    );
    Ok(())
}

#[test]
fn supplied_transaction_keeps_its_drop_behavior() -> Result<()> {
    let mut connection = Connection::open_in_memory()?;
    Store::from_conn(&connection).create()?;
    let mut transaction = connection.transaction()?;
    transaction.set_drop_behavior(DropBehavior::Commit);
    {
        let store = Store::from_tx(transaction);
        store.insert(5)?;
    }
    assert_eq!(Store::from_conn(&connection).items()?, vec![(5,)]);
    Ok(())
}

#[queries]
trait ForeignKeys {
    #[query = "INSERT INTO child VALUES (?1)"]
    fn insert(parent_id: i64);
}

#[test]
fn failed_commit_propagates_the_error_and_rolls_back() -> Result<()> {
    let mut connection = Connection::open_in_memory()?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE parent (id INTEGER PRIMARY KEY);
         CREATE TABLE child (
             parent_id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED
         );",
    )?;
    let transaction = ForeignKeys::from_tx(connection.transaction()?);
    transaction.insert(99)?;
    assert!(matches!(
        transaction.commit(),
        Err(Error::SqliteFailure(_, _))
    ));
    assert!(connection.is_autocommit());
    assert_eq!(
        connection.query_row("SELECT count(*) FROM child", [], |row| row.get::<_, i64>(0))?,
        0
    );
    Ok(())
}

#[test]
fn failed_rollback_propagates_the_error() -> Result<()> {
    let mut connection = Connection::open_in_memory()?;
    let transaction = Store::from_tx(connection.transaction()?);
    transaction.end_transaction()?;
    assert!(matches!(
        transaction.rollback(),
        Err(Error::SqliteFailure(_, _))
    ));
    assert!(connection.is_autocommit());
    Ok(())
}
