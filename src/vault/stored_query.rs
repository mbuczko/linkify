use crate::db::{DBLookupType, DBResult};
use crate::vault::auth::Authentication;
use crate::vault::Vault;

use crate::db::query::Query;
use miniserde::{Deserialize, Serialize};
use rusqlite::{params, OptionalExtension, Row};

#[derive(Serialize, Deserialize, Debug)]
pub struct StoredQuery {
    pub id: Option<i64>,
    pub name: String,
    pub query: String,
}

impl StoredQuery {
    pub fn new(id: Option<i64>, name: String, query: String) -> Self {
        StoredQuery { id, name, query }
    }
}

impl From<&Row<'_>> for StoredQuery {
    fn from(row: &Row) -> Self {
        StoredQuery::new(
            Some(row.get_unwrap(0)),
            row.get_unwrap::<_, String>(1),
            row.get_unwrap::<_, String>(2),
        )
    }
}

impl Vault {
    pub fn store_query(
        &self,
        auth: &Option<Authentication>,
        name: String,
        query: String,
    ) -> DBResult<i64> {
        let user = self.authenticate_user(auth)?;
        let mut conn = self.get_connection();
        let txn = conn.transaction().unwrap();
        txn.execute(
            "INSERT INTO queries(user_id, name, query) VALUES(?1, ?2, ?3) \
            ON CONFLICT(user_id, name) \
            DO UPDATE SET name = ?2, query = ?3, created_at = CURRENT_TIMESTAMP",
            params![user.id, name, query],
        )?;
        let id = match txn.last_insert_rowid() {
            0 => txn
                .query_row(
                    "SELECT id FROM queries WHERE user_id = ?1 AND name = ?2",
                    params![user.id, name],
                    |row| row.get(0),
                )
                .unwrap(),
            n => n,
        };
        txn.commit().and(Ok(id)).map_err(Into::into)
    }
    pub fn find_queries(
        &self,
        auth: &Option<Authentication>,
        name: Option<&str>,
        lookup_type: DBLookupType,
    ) -> DBResult<Vec<StoredQuery>> {
        let user = self.authenticate_user(auth)?;
        let name = name.map_or(Default::default(), |v| match lookup_type {
            DBLookupType::Exact => v.to_owned(),
            DBLookupType::Patterned => Query::patternize(v),
        });

        Query::new_with_initial(
            "SELECT s.id, name, query FROM queries s INNER JOIN users u ON s.user_id = u.id WHERE",
        )
        .concat_with_param("u.id = :id AND", (":id", &user.id))
        .concat_with_param(
            "name LIKE :name ORDER BY s.created_at DESC",
            (":name", &name),
        )
        .fetch(self.get_connection())
    }
    pub fn get_query(
        &self,
        auth: &Option<Authentication>,
        query_id: i64,
    ) -> DBResult<Option<StoredQuery>> {
        let user = self.authenticate_user(auth)?;
        let mut query = Query::new_with_initial("SELECT id, name, query FROM queries WHERE");
        query
            .concat_with_param("id = :sid AND", (":sid", &query_id))
            .concat_with_param("user_id = :uid", (":uid", &user.id));

        self.get_connection()
            .query_row(query.build().as_str(), query.named_params(), |r| {
                Ok(StoredQuery::from(r))
            })
            .optional()
            .map_err(Into::into)
    }
    pub fn del_query(
        &self,
        auth: &Option<Authentication>,
        query_id: i64,
    ) -> DBResult<Option<StoredQuery>> {
        match self.get_query(auth, query_id) {
            Ok(Some(query)) => {
                self.get_connection()
                    .execute("DELETE FROM queries WHERE id = ?", params![query.id])?;
                Ok(Some(query))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
