use crate::db::query::Query;
use crate::db::DBLookupType::{Exact, Patterned};
use crate::db::{DBLookupType, DBResult};
use crate::utils::{digest, path};
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
    pub id: Option<i64>,
    pub href: String,
    pub title: String,
    pub notes: Option<String>,
    pub tags: Option<Vec<Tag>>,
    pub hash: String,
    pub shared: bool,
    pub toread: bool,
    pub favourite: bool,
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _tags = self.tags.as_ref().map_or(None, |t| Some(t.join(" ")));
        let s = vec![self.href.as_str()];
        write!(f, "{}", s.join("\n"))
    }
}

impl Link {
    pub fn new(
        id: Option<i64>,
        href: &str,
        title: &str,
        notes: Option<&str>,
        tags: Option<Vec<Tag>>,
    ) -> Link {
        Link {
            id,
            href: href.to_string(),
            title: title.to_string(),
            notes: notes.map(Into::into),
            shared: false,
            toread: false,
            favourite: false,
            hash: digest(href, notes, &tags),
            tags,
        }
    }
    pub fn from_matches(matches: &ArgMatches) -> Link {
        let tags = matches
            .values_of("tags")
            .and_then(|t| Some(t.map(String::from).collect::<Vec<String>>()));

        Link::new(
            None,
            matches.value_of("url").unwrap_or_default(),
            matches.value_of("title").unwrap_or_default(),
            matches.value_of("notes"),
            tags,
        )
    }
    pub fn set_toread(mut self, toread: bool) -> Self {
        self.toread = toread;
        self
    }
    pub fn set_shared(mut self, shared: bool) -> Self {
        self.shared = shared;
        self
    }
    pub fn set_favourite(mut self, favourite: bool) -> Self {
        self.favourite = favourite;
        self
    }
}

impl Vault {
    fn store_link(&self, link: Link, user: &User) -> DBResult<Link> {
        let mut conn = self.get_connection();
        let txn = conn.transaction().unwrap();
        txn.execute(
            "INSERT INTO links(href, title, notes, hash, is_toread, is_shared, is_favourite, user_id) \
            VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
            ON CONFLICT(path(href), user_id) \
            DO UPDATE SET href = ?1, title = ?2, notes = ?3, hash = ?4, is_toread = ?5, is_shared = ?6, is_favourite = ?7",
            params![link.href, link.title, link.notes, link.hash, link.toread, link.shared, link.favourite, user.id],
        )?;
        let link_id: i64 = txn
            .query_row(
                "SELECT id FROM links WHERE href = ?1 AND user_id = ?2",
                params![link.href, user.id],
                |row| row.get(0),
            )
            .unwrap();

        // remove associations with tags assigned to given link so far
        txn.execute(
            "DELETE FROM links_tags WHERE link_id = ?1",
            params![link_id],
        )?;

        // join link with its tags (if provided)
        if let Some(vt) = &link.tags {
            let mut values: Vec<SqlValue> = Vec::new();
            for tag in vt {
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
                params![link_id, Rc::new(values), user.id],
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
        let mut query = Query::new_with_initial(
            "SELECT l.id, href, title, notes, group_concat(tag) AS tagz, is_toread, is_shared, is_favourite \
            FROM links l \
            LEFT JOIN links_tags lt ON l.id = lt.link_id \
            LEFT JOIN tags t ON lt.tag_id = t.id WHERE",
        );

        let tags = pattern.tags.to_owned().unwrap_or_default();
        let path = path(pattern.href.as_str());
        let title = Query::patternize(&pattern.title);
        let limit = limit.unwrap_or(0);

        // Searching by title and notes is equivalent. Also, when href was not not explicitly
        // provided it's equivalent to title. This is to easily find a link by either a title
        // or some part of url.

        if !title.is_empty() {
            if path.is_empty() {
                query.concat_with_param(
                    "(title LIKE :title OR href LIKE :title OR notes LIKE :title) AND",
                    (":title", &title),
                );
            } else {
                query.concat_with_param(
                    "(title LIKE :title OR notes LIKE :title) AND",
                    (":title", &title),
                );
            }
        }
        let href = match lookup_type {
            Exact => path,
            Patterned => Query::patternize(&path),
        };
        if !href.is_empty() {
            query.concat_with_param("path(href) LIKE :href AND", (":href", &href));
        }
        query.concat_with_param("l.user_id = :id GROUP BY l.id", (":id", &user.id));

        // Tags are classified as: optional, +required and -excluded.
        //
        // Each classification follows different rule to decide whether to include or exclude
        // link from results. And so, for any given link with tags attached, to add link to
        // final results:
        //
        // - at least one of optional tags needs to attached to the link
        // - all of required tags need to be attached to the link
        // - all of excluded tags need to be missing
        //
        // Rules can be combined together when tags of different classification are used in a query,
        // eg. "tags:rust,programming,-hyper,+server" means that all links tagged either with "rust"
        // or "programming", having no "hyper" tag and having "server" tag should be returned.

        let (optional, required, excluded) = Vault::classify_tags(tags);

        let has_optional = !optional.is_empty();
        let has_required = !required.is_empty();
        let has_excluded = !excluded.is_empty();

        let excluded = excluded.join(",");
        let required = required.join(",");

        let optional_ptr = Rc::new(optional.into_iter().map(SqlValue::from).collect());
        if has_optional || has_required || has_excluded {
            query.concat("HAVING");

            if has_optional {
                query.concat_with_param(
                    "l.id IN (\
            SELECT link_id FROM links_tags lt2 \
            JOIN tags t2 ON lt2.tag_id = t2.id AND t2.tag IN rarray(:tags)) AND",
                    (":tags", &optional_ptr),
                );
            }
            if has_required {
                query.concat_with_param(
                    "LENGTH(tagz) > 0 AND every(tagz, :reqs) AND",
                    (":reqs", &required),
                );
            }
            if has_excluded {
                query.concat_with_param(
                    "LENGTH(tagz) > 0 AND some(tagz, :excls) = FALSE AND",
                    (":excls", &excluded),
                );
            }
            query.concat("1=1");
        }
        query.concat("ORDER BY l.created_at DESC");

        // Finally the limit. It's not the best idea to return all the links if no constraints
        // were provided. Let's limit result to 10 links by default.

        if limit > 0 {
            query.concat_with_param("LIMIT :limit", (":limit", &limit));
        }

        let conn = self.get_connection();
        let mut stmt = conn.prepare(query.to_string().as_str())?;
        let rows = stmt.query_map_named(query.named_params(), |row| {
            Ok(Link::new(
                Some(row.get_unwrap(0)),
                &row.get_unwrap::<_, String>(1),
                &row.get_unwrap::<_, String>(2),
                row.get::<_, String>(3).ok().as_deref(),
                row.get::<_, String>(4)
                    .map_or(Some(Default::default()), |t| {
                        Some(t.split(',').map(String::from).collect())
                    }),
            )
            .set_toread(row.get_unwrap::<_, bool>(5))
            .set_shared(row.get_unwrap::<_, bool>(6))
            .set_favourite(row.get_unwrap::<_, bool>(7)))
        })?;
        Result::from_iter(rows).map_err(Into::into)
    }
    pub fn get_href(&self, auth: Option<Authentication>, link_id: i64) -> DBResult<String> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let href = self.get_connection().query_row(
            "SELECT href FROM links WHERE id = ?1 AND user_id = ?2",
            params![link_id, user.id],
            |row| row.get::<_, String>(0),
        )?;
        Ok(href)
    }
    pub fn get_link(&self, auth: Option<Authentication>, href: &str) -> DBResult<Option<Link>> {
        let pattern = Link::new(None, &href, "", None, None);
        self.find_links(auth, pattern, DBLookupType::Exact, Some(1))
            .and_then(|v| Ok(v.first().cloned()))
    }
    pub fn del_link(&self, auth: Option<Authentication>, href: &str) -> DBResult<Option<Link>> {
        match self.get_link(auth, &href) {
            Ok(Some(link)) => {
                self.get_connection()
                    .execute("DELETE FROM links WHERE id = ?", params![link.id])?;
                Ok(Some(link))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
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
            None,
            href.last().map_or("", |v| v.trim()),
            title.join("%").trim(),
            Some(&notes.join("%").trim()),
            Some(tags),
        );
        self.match_links(auth, link, limit)
    }
}
