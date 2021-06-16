pub mod query;

use super::utils::{every, path, some};

use failure::Fail;
use log::debug;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::functions::FunctionFlags;
use rusqlite::vtab::array;
use rusqlite::{Connection, Error as SqliteError};
use std::path::Path;

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

    #[fail(display = "Incorrect version")]
    BadVersion,
}

/// Lookup type for core entities, like users and links
pub enum DBLookupType {
    /// Exact lookup, searches for entity based on exact phrase
    Exact,
    /// Substring based lookup
    Patterned,
}

pub type DBResult<T> = Result<T, DBError>;

impl From<SqliteError> for DBError {
    fn from(err: SqliteError) -> Self {
        DBError::Sqlite(err)
    }
}

fn add_functions(conn: &Connection) -> Result<(), DBError> {
    conn.create_scalar_function(
        "path",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            let url = ctx.get::<String>(0)?;
            Ok(path(&url))
        },
    )?;
    conn.create_scalar_function(
        "every",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            let elements = ctx.get::<String>(0)?;
            let expected = ctx.get::<String>(1)?;
            Ok(every(&elements, &expected))
        },
    )?;
    conn.create_scalar_function(
        "some",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            let elements = ctx.get::<String>(0)?;
            let expected = ctx.get::<String>(1)?;
            Ok(some(&elements, &expected))
        },
    )?;
    Ok(())
}

pub fn conn_manager<P: AsRef<Path>>(db: P) -> SqliteConnectionManager {
    debug!("Opening database ({})", db.as_ref().display());

    let scm: SqliteConnectionManager = SqliteConnectionManager::file(db);
    scm.with_init(|c| {
        add_functions(c).expect("Cannot initialize additional SQLite functions");
        array::load_module(c).unwrap();
        c.execute_batch("PRAGMA foreign_keys=1; PRAGMA busy_timeout=3000;")
    })
}
