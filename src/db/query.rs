use crate::db::DBResult;

use failure::_core::iter::FromIterator;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Row, ToSql};

pub struct Query<'a> {
    params: Vec<(&'a str, &'a dyn ToSql)>,
    query: Vec<&'a str>,
}

impl<'a> Query<'a> {
    pub fn new() -> Self {
        Query {
            params: Vec::with_capacity(4),
            query: Vec::with_capacity(4),
        }
    }
    pub fn new_with_initial(initial_query: &'static str) -> Self {
        let mut q = Query::new();
        q.query.push(initial_query);
        q
    }
    pub fn concat(&mut self, query_str: &'static str) -> &mut Self {
        self.query.push(query_str);
        self
    }
    pub fn concat_with_param(
        &mut self,
        query_str: &'static str,
        query_param: (&'a str, &'a dyn ToSql),
    ) -> &mut Self {
        self.params.push(query_param);
        self.concat(query_str)
    }
    pub fn build(&self) -> String {
        self.query.join(" ")
    }
    pub fn named_params(&self) -> &[(&str, &dyn ToSql)] {
        self.params.as_slice()
    }
    pub fn patternize(arg: &str) -> String {
        format!("%{}%", arg)
    }
    pub fn fetch_as<T, F>(
        &self,
        conn: PooledConnection<SqliteConnectionManager>,
        f: F,
    ) -> DBResult<Vec<T>>
    where
        F: Fn(&Row) -> T,
    {
        let mut stmt = conn.prepare(&self.build())?;
        let rows = stmt.query_map_named(self.named_params(), |row| Ok(f(row)))?;

        Result::from_iter(rows).map_err(Into::into)
    }

    pub fn fetch<T>(&self, conn: PooledConnection<SqliteConnectionManager>) -> DBResult<Vec<T>>
    where
        T: for<'q> From<&'q Row<'q>>,
    {
        self.fetch_as(conn, |row| T::from(row))
    }
}
