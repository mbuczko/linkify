use crate::db::{get_query_results, DBLookupType, DBResult};
use crate::vault::auth::Authentication;
use crate::vault::Vault;

use crate::db::query::Query;
use miniserde::{Deserialize, Serialize};
use rusqlite::{params, OptionalExtension, Row};

#[derive(Serialize, Deserialize, Debug)]
pub struct Search {
    pub id: Option<i64>,
    pub name: String,
    pub query: String,
}

impl Search {
    pub fn new(id: Option<i64>, name: String, query: String) -> Self {
        Search { id, name, query }
    }
}

impl From<&Row<'_>> for Search {
    fn from(row: &Row) -> Self {
        Search::new(
            Some(row.get_unwrap(0)),
            row.get_unwrap::<_, String>(1),
            row.get_unwrap::<_, String>(2),
        )
    }
}

impl Vault {
    pub fn store_search(
        &self,
        auth: Option<Authentication>,
        name: String,
        query: String,
    ) -> DBResult<i64> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };

        let mut conn = self.get_connection();
        let txn = conn.transaction().unwrap();
        txn.execute(
            "INSERT INTO searches(user_id, name, query) VALUES(?1, ?2, ?3) \
            ON CONFLICT(user_id, name) \
            DO UPDATE SET name = ?2, query = ?3, created_at = CURRENT_TIMESTAMP",
            params![user.id, name, query],
        )?;
        let id = match txn.last_insert_rowid() {
            0 => txn
                .query_row(
                    "SELECT id FROM searches WHERE user_id = ?1 AND name = ?2",
                    params![user.id, name],
                    |row| row.get(0),
                )
                .unwrap(),
            n => n,
        };
        txn.commit().and(Ok(id)).map_err(Into::into)
    }
    pub fn find_searches(
        &self,
        auth: Option<Authentication>,
        name: Option<&str>,
        lookup_type: DBLookupType,
    ) -> DBResult<Vec<Search>> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let name = name.map_or(Default::default(), |v| match lookup_type {
            DBLookupType::Exact => v.to_owned(),
            DBLookupType::Patterned => Query::patternize(v),
        });
        let mut query = Query::new_with_initial(
            "SELECT s.id, name, query FROM searches s INNER JOIN users u ON s.user_id = u.id WHERE",
        );
        query
            .concat_with_param("u.id = :id AND", (":id", &user.id))
            .concat_with_param(
                "name LIKE :name ORDER BY s.created_at DESC",
                (":name", &name),
            );
        get_query_results(self.get_connection(), query)
    }
    pub fn get_search(
        &self,
        auth: Option<Authentication>,
        search_id: i64,
    ) -> DBResult<Option<Search>> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let mut query = Query::new_with_initial("SELECT id, name, query FROM searches WHERE");
        query
            .concat_with_param("id = :sid AND", (":sid", &search_id))
            .concat_with_param("user_id = :uid", (":uid", &user.id));

        self.get_connection()
            .query_row_named(query.to_string().as_str(), query.named_params(), |r| {
                Ok(Search::from(r))
            })
            .optional()
            .map_err(Into::into)
    }
    pub fn del_search(
        &self,
        auth: Option<Authentication>,
        search_id: i64,
    ) -> DBResult<Option<Search>> {
        match self.get_search(auth, search_id) {
            Ok(Some(search)) => {
                self.get_connection()
                    .execute("DELETE FROM searches WHERE id = ?", params![search.id])?;
                Ok(Some(search))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
