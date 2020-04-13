use crate::db::{DBLookupType, DBResult};
use crate::vault::auth::Authentication;
use crate::vault::Vault;

use crate::db::query::Query;
use miniserde::{Deserialize, Serialize};
use rusqlite::params;
use std::iter::FromIterator;

#[derive(Serialize, Deserialize, Debug)]
pub struct Search {
    pub name: String,
    pub query: String,
}

impl Search {
    pub fn new(name: String, query: String) -> Self {
        Search { name, query }
    }
}
impl Vault {
    pub fn store_search(
        &self,
        auth: &Option<Authentication>,
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
        auth: &Option<Authentication>,
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
            "SELECT name, query FROM searches s INNER JOIN users u ON s.user_id = u.id WHERE",
        );
        query
            .concat_with_param("u.id = :id AND", (":id", &user.id))
            .concat_with_param(
                "name LIKE :name  ORDER BY s.created_at DESC",
                (":name", &name),
            );

        let conn = self.get_connection();
        let mut stmt = conn.prepare(query.to_string().as_str())?;
        let rows = stmt.query_map_named(query.named_params(), |row| {
            Ok(Search::new(
                row.get_unwrap::<_, String>(0),
                row.get_unwrap::<_, String>(1),
            ))
        })?;
        Result::from_iter(rows).map_err(Into::into)
    }
}
