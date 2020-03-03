use rusqlite::types::ToSql;
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

pub struct Query<'a> {
    params: Vec<(&'a str, &'a dyn ToSql)>,
    query: Vec<&'a str>,
}

impl From<SqliteError> for DBError {
    fn from(err: SqliteError) -> Self {
        DBError::Sqlite(err)
    }
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
    pub fn concat(&mut self, query_str: &'static str) -> &Self {
        self.query.push(query_str);
        self
    }
    pub fn concat_with_param(
        &mut self,
        query_str: &'static str,
        query_param: (&'a str, &'a dyn ToSql),
    ) -> &Self {
        self.params.push(query_param);
        self.concat(query_str)
    }
    pub fn to_string(&self) -> String {
        self.query.join(" ")
    }
    pub fn named_params(&self) -> &[(&str, &dyn ToSql)] {
        self.params.as_slice()
    }
    pub fn patternize(arg: &str) -> Option<String> {
        if arg.is_empty() {
            None
        } else {
            Some(format!("%{}%", arg))
        }
    }
}
