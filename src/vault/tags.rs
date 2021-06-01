use crate::db::query::Query;
use crate::db::DBResult;
use crate::utils::remove_first;
use crate::vault::auth::Authentication;
use crate::vault::Vault;

use rusqlite::types::Value as SqlValue;
use std::rc::Rc;

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
        auth: &Option<Authentication>,
        pattern: Option<&str>,
        exclude: Option<Vec<Tag>>,
        limit: Option<u16>,
    ) -> DBResult<Vec<Tag>> {
        let user = self.authenticate_user(auth)?;
        let pattern = Query::patternize(pattern.unwrap_or_default());
        let excludes = Rc::<Vec<_>>::new(
            exclude
                .unwrap_or_default()
                .into_iter()
                .map(SqlValue::from)
                .collect(),
        );
        let limit = limit.unwrap_or(8);

        Query::new_with_initial("SELECT tag FROM tags")
            .concat_with_param("WHERE user_id = :id AND", (":id", &user.id))
            .concat_with_param("tag LIKE :pattern AND", (":pattern", &pattern))
            .concat_with_param("tag NOT IN rarray(:excludes)", (":excludes", &excludes))
            .concat_with_param("ORDER BY used_at DESC LIMIT :limit", (":limit", &limit))
            .fetch_as(self.get_connection(), |row| row.get_unwrap::<_, Tag>(0))
    }
}
