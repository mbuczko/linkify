pub mod query;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::vtab::array;
use rusqlite::Error as SqliteError;
use failure::Fail;

#[derive(Debug, Fail)]
pub enum DBError {
    #[fail(display = "Database error")]
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

pub fn conn_manager(db: &str) -> SqliteConnectionManager {
    SqliteConnectionManager::file(db).with_init(|c| {
        array::load_module(c).unwrap();
        c.execute_batch("PRAGMA foreign_keys=1;")
    })
}
