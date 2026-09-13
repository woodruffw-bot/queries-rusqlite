//! Positional decoding for each supported tuple length.

use queries_rusqlite::FromRow;
use rusqlite::{Connection, Error, Result};

macro_rules! integer {
    ($name:ident) => {
        i64
    };
}

macro_rules! check_tuple {
    ($connection:expr; $($name:ident:$index:literal),+) => {{
        let mut columns = vec![$($index.to_string()),+];
        let sql = format!("SELECT {}", columns.join(", "));
        let values: ($(integer!($name),)+) =
            $connection.query_row(&sql, [], FromRow::from_row)?;
        let ($($name,)+) = values;
        assert_eq!([$($name),+], [$($index),+]);

        let last = columns.len() - 1;
        columns[last] = "'not an integer'".to_owned();
        let sql = format!("SELECT {}", columns.join(", "));
        let error = $connection.query_row::<($(integer!($name),)+), _, _>(
            &sql, [], FromRow::from_row,
        );
        assert!(matches!(error, Err(Error::InvalidColumnType(index, _, _)) if index == last));
    }};
}

#[test]
fn tuples_decode_in_order_and_propagate_conversion_errors() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    check_tuple!(connection; a:0);
    check_tuple!(connection; a:0, b:1);
    check_tuple!(connection; a:0, b:1, c:2);
    check_tuple!(connection; a:0, b:1, c:2, d:3);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8, j:9);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8, j:9, k:10);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8, j:9, k:10, l:11);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8, j:9, k:10, l:11, m:12);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8, j:9, k:10, l:11, m:12, n:13);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8, j:9, k:10, l:11, m:12, n:13, o:14);
    check_tuple!(connection; a:0, b:1, c:2, d:3, e:4, f:5, g:6, h:7, i:8, j:9, k:10, l:11, m:12, n:13, o:14, p:15);
    Ok(())
}
