pub mod query;
pub mod migrations;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::vtab::array;
use rusqlite::Error as SqliteError;

#[derive(Debug)]
pub enum DBError {
    Sqlite(SqliteError),
    Unauthenticated,
    UnknownUser,
    BadPassword,
}

pub enum DBSeachType {
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
