use crate::db::query::Query;
use crate::db::DBResult;
use crate::vault::auth::Authentication;
use crate::vault::Vault;

use crate::utils::remove_first;
use std::iter::FromIterator;

pub type Tag = String;

impl Vault {
    pub fn classify_tags(tags: Vec<Tag>) -> (Vec<Tag>, Vec<Tag>, Vec<Tag>) {
        let mut optional = Vec::new();
        let mut required = Vec::new();
        let mut excluded = Vec::new();

        for t in tags {
            if t.starts_with('+') {
                if let Some(s) = remove_first(t.as_str()) {
                    required.push(s.to_string());
                }
            } else if t.starts_with('-') {
                if let Some(s) = remove_first(t.as_str()) {
                    excluded.push(s.to_string());
                }
            } else {
                optional.push(t);
            }
        }
        (optional, required, excluded)
    }
    pub fn recent_tags(
        &self,
        auth: Option<Authentication>,
        pattern: Option<&str>,
        limit: Option<u16>,
    ) -> DBResult<Vec<Tag>> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let pattern = Query::patternize(pattern.unwrap_or_default());
        let limit = limit.unwrap_or(8);
        let mut query = Query::new_with_initial("SELECT tag FROM tags");
        query
            .concat_with_param("WHERE user_id = :id AND", (":id", &user.id))
            .concat_with_param("tag LIKE :pattern", (":pattern", &pattern))
            .concat_with_param("ORDER BY used_at DESC LIMIT :limit", (":limit", &limit));

        let conn = self.get_connection();
        let mut stmt = conn.prepare(query.to_string().as_str())?;
        let rows =
            stmt.query_map_named(query.named_params(), |row| Ok(row.get_unwrap::<_, Tag>(0)))?;

        Result::from_iter(rows).map_err(Into::into)
    }
}
