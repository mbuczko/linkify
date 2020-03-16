use crate::db::DBResult;
use crate::utils::digest;
use crate::vault::auth::Authentication;
use crate::vault::user::User;
use crate::vault::Vault;

use crate::db::query::Query;
use clap::ArgMatches;
use miniserde::{Deserialize, Serialize};
use rusqlite::params;
use rusqlite::types::Value as SqlValue;
use std::fmt;
use std::iter::FromIterator;
use std::rc::Rc;

type Tag = String;

#[derive(Serialize, Deserialize, Debug)]
pub struct Link {
    pub href: String,
    pub description: Option<String>,
    pub tags: Option<Vec<Tag>>,
    pub hash: String,
    pub shared: bool,
    pub toread: bool,
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _tags = self.tags.as_ref().map_or(None, |t| Some(t.join(" ")));
        let s = vec![self.href.as_str()];
        //        if let Some(d) = self.description.as_ref() {
        //            s.push(d);
        //        }
        //        if let Some(t) = tags.as_ref() {
        //            s.push("--");
        //            s.push(t);
        //        }
        write!(f, "{}", s.join("\n"))
    }
}

impl Link {
    pub fn new(href: &str, description: Option<&str>, tags: Option<Vec<Tag>>) -> Link {
        Link {
            href: href.to_string(),
            description: description.map(Into::into),
            hash: digest(href, &description, &tags),
            shared: false,
            toread: false,
            tags,
        }
    }
    pub fn from(matches: &ArgMatches) -> Link {
        let tags = matches
            .values_of("tags")
            .and_then(|t| Some(t.map(String::from).collect::<Vec<String>>()));

        Link::new(
            matches.value_of("url").unwrap_or_default(),
            matches.value_of("description"),
            tags,
        )
    }
}

impl Vault {
    fn store_link(&self, link: &Link, user: &User) -> DBResult<i64> {
        let mut conn = self.get_connection();
        let txn = conn.transaction().unwrap();
        txn.execute(
            "INSERT INTO links(href, description, hash, user_id) VALUES(?1, ?2, ?3, ?4) \
            ON CONFLICT(href, user_id) \
            DO UPDATE SET description = ?2, hash = ?3",
            params![link.href, link.description, link.hash, user.id],
        )?;

        // note that last_insert_rowid returns 0 for already existing URLs
        let id = match txn.last_insert_rowid() {
            0 => txn
                .query_row(
                    "SELECT id FROM links WHERE href = ?1 AND user_id = ?2",
                    params![link.href, user.id],
                    |row| row.get(0),
                )
                .unwrap(),
            n => n,
        };

        // remove tags associated so far
        txn.execute("DELETE FROM links_tags WHERE link_id = ?1", params![id])?;

        // join link with its tags (if provided)
        if let Some(tv) = &link.tags {
            let mut values: Vec<SqlValue> = Vec::new();
            for tag in tv {
                txn.execute(
                    "INSERT INTO tags(tag, user_id) VALUES(?1, ?2) \
            ON CONFLICT(tag, user_id) \
            DO UPDATE SET used_at = CURRENT_TIMESTAMP",
                    params![tag, user.id],
                )?;
                values.push(SqlValue::from(tag.to_string()));
            }
            txn.execute(
                "INSERT INTO links_tags(link_id, tag_id) \
        SELECT ?1, id FROM tags WHERE tag IN rarray(?2) AND user_id = ?3",
                params![id, Rc::new(values), user.id],
            )?;
        }
        txn.commit().and(Ok(id)).map_err(Into::into)
    }
    pub fn add_link(&self, link: &Link, auth: &Option<Authentication>) -> DBResult<i64> {
        match self.authenticate_user(auth) {
            Ok(u) => self.store_link(link, &u),
            Err(e) => return Err(e),
        }
    }
    pub fn del_link(&self, link: &Link, auth: &Option<Authentication>) -> DBResult<i64> {
        match self.authenticate_user(auth) {
            Ok(u) => {
                let link_id = self.get_connection().query_row(
                    "SELECT id FROM LINKS WHERE href = ? AND user_id = ?",
                    params![&link.href, u.id],
                    |row| row.get::<_, i64>(0),
                )?;
                self.get_connection()
                    .execute("DELETE FROM links WHERE id = ?", params![link_id])?;
                Ok(link_id)
            }
            Err(e) => return Err(e),
        }
    }
    pub fn import_links(&self, links: Vec<Link>, auth: &Option<Authentication>) -> DBResult<u32> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let mut imported: u32 = 0;
        for link in links {
            if self.store_link(&link, &user).is_ok() {
                imported += 1;
                println!("+ {}", link.href)
            }
        }
        Ok(imported)
    }
    pub fn match_links(&self, link: &Link, auth: &Option<Authentication>) -> DBResult<Vec<Link>> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let tags = link.tags.to_owned().unwrap_or_default();
        let ptr = Rc::new(tags.into_iter().map(SqlValue::from).collect());
        let href = Query::patternize(&link.href).unwrap_or_default();
        let desc = link
            .description
            .as_ref()
            .map_or(None, |v| Query::patternize(v))
            .unwrap_or_default();

        let mut query = Query::new_with_initial(
            "SELECT href, description, group_concat(tag) FROM links l \
        LEFT JOIN links_tags lt ON l.id = lt.link_id \
        LEFT JOIN tags t ON lt.tag_id = t.id \
        WHERE",
        );
        if !href.is_empty() {
            query.concat_with_param("href LIKE :href AND", (":href", &href));
        }
        if !desc.is_empty() {
            query.concat_with_param("description LIKE :desc AND", (":desc", &desc));
        }

        query.concat_with_param("l.user_id = :id", (":id", &user.id));
        query.concat("GROUP BY l.id");

        if link.tags.is_some() {
            query.concat_with_param(
                "HAVING l.id IN \
            (SELECT link_id FROM links_tags lt2 \
            JOIN tags t2 ON lt2.tag_id = t2.id AND t2.tag IN rarray(:tags))",
                (":tags", &ptr),
            );
        }
        query.concat("ORDER BY l.created_at DESC");

        let conn = self.get_connection();
        let mut stmt = conn.prepare(query.to_string().as_str())?;
        let rows = stmt.query_map_named(query.named_params(), |row| {
            Ok(Link::new(
                &row.get_unwrap::<_, String>(0),
                row.get::<_, String>(1).ok().as_deref(),
                row.get::<_, String>(2)
                    .map_or(None, |t| Some(t.split('.').map(String::from).collect())),
            ))
        })?;
        Result::from_iter(rows).map_err(Into::into)
    }
}
