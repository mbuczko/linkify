pub mod query;

use crate::utils::{every, path, some};
use failure::Fail;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::vtab::array;
use rusqlite::{Connection, Error as SqliteError};

#[derive(Debug, Fail)]
pub enum DBError {
    #[fail(display = "Database internal error")]
    Sqlite(SqliteError),

    #[fail(display = "Unauthenticated request")]
    Unauthenticated,

    #[fail(display = "Unknown user")]
    UnknownUser,

    #[fail(display = "Bad credentials")]
    BadPassword,
}

pub enum DBLookupType {
    Exact,
    Patterned,
}

pub type DBResult<T> = Result<T, DBError>;

impl From<SqliteError> for DBError {
    fn from(err: SqliteError) -> Self {
        DBError::Sqlite(err)
    }
}

fn add_path_function(conn: &Connection) -> Result<(), DBError> {
    conn.create_scalar_function("path", 1, true, move |ctx| {
        let url = ctx.get::<String>(0)?;
        Ok(path(&url))
    })?;
    conn.create_scalar_function("every", 2, true, move |ctx| {
        let elements = ctx.get::<String>(0)?;
        let expected = ctx.get::<String>(1)?;
        Ok(every(&elements, &expected))
    })?;
    conn.create_scalar_function("some", 2, true, move |ctx| {
        let elements = ctx.get::<String>(0)?;
        let expected = ctx.get::<String>(1)?;
        Ok(some(&elements, &expected))
    })?;
    Ok(())
}

pub fn conn_manager(db: &str) -> SqliteConnectionManager {
    SqliteConnectionManager::file(db).with_init(|c| {
        add_path_function(c).expect("Cannot initialize SQLite function");
        array::load_module(c).unwrap();
        c.execute_batch("PRAGMA foreign_keys=1;")
    })
}
