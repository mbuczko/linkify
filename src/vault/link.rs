use crate::db::query::Query;
use crate::db::DBLookupType::{Exact, Patterned};
use crate::db::{DBLookupType, DBResult};
use crate::utils::digest;
use crate::vault::auth::Authentication;
use crate::vault::tags::Tag;
use crate::vault::user::User;
use crate::vault::Vault;

use clap::ArgMatches;
use miniserde::{Deserialize, Serialize};
use rusqlite::params;
use rusqlite::types::Value as SqlValue;
use std::fmt;
use std::iter::FromIterator;
use std::rc::Rc;

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Link {
    pub href: String,
    pub title: String,
    pub notes: Option<String>,
    pub tags: Option<Vec<Tag>>,
    pub hash: String,
    pub shared: bool,
    pub toread: bool,
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _tags = self.tags.as_ref().map_or(None, |t| Some(t.join(" ")));
        let s = vec![self.href.as_str()];
        write!(f, "{}", s.join("\n"))
    }
}

impl Link {
    pub fn new(href: &str, title: &str, notes: Option<&str>, tags: Option<Vec<Tag>>) -> Link {
        Link {
            href: href.to_string(),
            title: title.to_string(),
            notes: notes.map(Into::into),
            hash: digest(href, &notes, &tags),
            shared: false,
            toread: false,
            tags,
        }
    }
    pub fn from_matches(matches: &ArgMatches) -> Link {
        let tags = matches
            .values_of("tags")
            .and_then(|t| Some(t.map(String::from).collect::<Vec<String>>()));

        Link::new(
            matches.value_of("url").unwrap_or_default(),
            matches.value_of("title").unwrap_or_default(),
            matches.value_of("notes"),
            tags,
        )
    }
}

impl From<&str> for Link {
    fn from(href: &str) -> Link {
        Link::new(href, "", None, None)
    }
}

impl Vault {
    fn store_link(&self, link: Link, user: &User) -> DBResult<Link> {
        let mut conn = self.get_connection();
        let txn = conn.transaction().unwrap();
        txn.execute(
            "INSERT INTO links(href, title, notes, hash, user_id) VALUES(?1, ?2, ?3, ?4, ?5) \
            ON CONFLICT(path(href), user_id) \
            DO UPDATE SET href = ?1, title = ?2, notes = ?3, hash = ?4",
            params![link.href, link.title, link.notes, link.hash, user.id],
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

        // remove associations with tags assigned to given link so far
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
        txn.commit().and(Ok(link)).map_err(Into::into)
    }
    pub fn add_link(&self, auth: Option<Authentication>, link: Link) -> DBResult<Link> {
        match self.authenticate_user(auth) {
            Ok(u) => self.store_link(link, &u),
            Err(e) => return Err(e),
        }
    }
    pub fn del_link(&self, auth: Option<Authentication>, link: Link) -> DBResult<Link> {
        match self.authenticate_user(auth) {
            Ok(u) => {
                let link_id = self.get_connection().query_row(
                    "SELECT id FROM LINKS WHERE href = ? AND user_id = ?",
                    params![&link.href, u.id],
                    |row| row.get::<_, i64>(0),
                )?;
                self.get_connection()
                    .execute("DELETE FROM links WHERE id = ?", params![link_id])?;
                Ok(link)
            }
            Err(e) => return Err(e),
        }
    }
    pub fn import_links(&self, auth: Option<Authentication>, links: Vec<Link>) -> DBResult<u32> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let mut imported: u32 = 0;
        for link in links {
            if let Ok(l) = self.store_link(link, &user) {
                imported += 1;
                println!("+ {}", l.href)
            }
        }
        Ok(imported)
    }
    pub fn find_links(
        &self,
        auth: Option<Authentication>,
        pattern: Link,
        lookup_type: DBLookupType,
        limit: Option<u16>,
    ) -> DBResult<Vec<Link>> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let tags = pattern.tags.to_owned().unwrap_or_default();
        let href = match lookup_type {
            Exact => pattern.href.to_owned(),
            Patterned => Query::patternize(&pattern.href),
        };
        let title = Query::patternize(&pattern.title);
        let limit = limit.unwrap_or(0);
        let notes = pattern
            .notes
            .as_ref()
            .map_or(Default::default(), |v| Query::patternize(v));

        let mut query = Query::new_with_initial(
            "SELECT href, title, notes, group_concat(tag) FROM links l \
        LEFT JOIN links_tags lt ON l.id = lt.link_id \
        LEFT JOIN tags t ON lt.tag_id = t.id WHERE",
        );

        if !href.is_empty() {
            query.concat_with_param("href LIKE :href AND", (":href", &href));
        }
        if !notes.is_empty() {
            query.concat_with_param("notes LIKE :notes AND", (":notes", &notes));
        }
        if !title.is_empty() {
            if pattern.href.is_empty() {
                query.concat_with_param(
                    "(title LIKE :title OR href LIKE :title) AND",
                    (":title", &title),
                );
            } else {
                query.concat_with_param("title LIKE :title AND", (":title", &title));
            }
        }
        query.concat_with_param("l.user_id = :id GROUP BY l.id", (":id", &user.id));

        let has_tags = !tags.is_empty();
        let ptr = Rc::new(tags.into_iter().map(SqlValue::from).collect());

        if has_tags {
            query.concat_with_param(
                "HAVING l.id IN \
            (SELECT link_id FROM links_tags lt2 \
            JOIN tags t2 ON lt2.tag_id = t2.id AND t2.tag IN rarray(:tags))",
                (":tags", &ptr),
            );
        }
        query.concat("ORDER BY l.created_at DESC");
        if limit > 0 {
            query.concat_with_param("LIMIT :limit", (":limit", &limit));
        }
        let conn = self.get_connection();
        let mut stmt = conn.prepare(query.to_string().as_str())?;
        let rows = stmt.query_map_named(query.named_params(), |row| {
            Ok(Link::new(
                &row.get_unwrap::<_, String>(0),
                &row.get_unwrap::<_, String>(1),
                row.get::<_, String>(2).ok().as_deref(),
                row.get::<_, String>(3)
                    .map_or(None, |t| Some(t.split(',').map(String::from).collect())),
            ))
        })?;
        Result::from_iter(rows).map_err(Into::into)
    }
    pub fn match_links(
        &self,
        auth: Option<Authentication>,
        pattern: Link,
        limit: Option<u16>,
    ) -> DBResult<Vec<Link>> {
        self.find_links(auth, pattern, DBLookupType::Patterned, limit)
    }
    pub fn query(
        &self,
        auth: Option<Authentication>,
        q: String,
        limit: Option<u16>,
    ) -> DBResult<Vec<Link>> {
        let mut href: Vec<&str> = Vec::new();
        let mut title: Vec<&str> = Vec::new();
        let mut notes: Vec<&str> = Vec::new();
        let mut tags: Vec<String> = Vec::new();

        for chunk in q.split_whitespace() {
            let ch: Vec<_> = chunk.split(':').collect();
            if ch.len() == 2 {
                match ch[0] {
                    "tags" => {
                        let more_tags = ch[1].split(',').map(String::from).collect::<Vec<String>>();
                        tags.extend(more_tags);
                    }
                    "href" => href.push(ch[1]),
                    "notes" => notes.push(ch[1]),
                    _ => title.push(chunk),
                }
            } else {
                title.push(chunk);
            }
        }
        let link = Link::new(
            href.last().map_or("", |v| v.trim()),
            title.join("%").trim(),
            Some(&notes.join("%").trim()),
            Some(tags),
        );
        self.match_links(auth, link, limit)
    }
}
