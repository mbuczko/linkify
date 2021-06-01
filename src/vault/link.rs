use crate::db::query::Query;
use crate::db::DBError::BadVersion;
use crate::db::DBLookupType::{Exact, Patterned};
use crate::db::{DBLookupType, DBResult};
use crate::utils::path;
use crate::vault::auth::Authentication;
use crate::vault::tags::Tag;
use crate::vault::user::User;
use crate::vault::Vault;

use clap::ArgMatches;
use miniserde::{Deserialize, Serialize};
use rusqlite::{params, Row};
use rusqlite::{types::Value as SqlValue, Transaction};
use sha1::Sha1;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Version(i32);

impl Version {
    pub fn new(offset: i32) -> Version {
        Version(offset)
    }
    pub fn offset(&self) -> i32 {
        self.0
    }
    pub fn unknown() -> Version {
        Version(-1)
    }
    pub fn bump(&self) -> Self {
        Version(self.offset() + 1)
    }
    pub fn is_valid(&self) -> bool {
        self.offset() >= 0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.offset())
    }
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Link {
    pub id: Option<i64>,
    pub href: String,
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<Tag>>,
    pub hash: Option<String>,
    pub shared: bool,
    pub toread: bool,
    pub favourite: bool,
    pub created_at: String,
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _tags = self.tags.as_ref().map(|t| t.join(" "));
        let s = vec![self.href.as_str()];
        write!(f, "{}", s.join("\n"))
    }
}

impl From<&Row<'_>> for Link {
    fn from(row: &Row) -> Self {
        Link::new(
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
        .set_favourite(row.get_unwrap::<_, bool>(7))
        .set_timestamp(row.get_unwrap::<_, String>(8))
    }
}

impl Link {
    pub fn new(
        id: Option<i64>,
        href: &str,
        name: &str,
        description: Option<&str>,
        tags: Option<Vec<Tag>>,
    ) -> Link {
        Link {
            id,
            href: href.to_string(),
            name: name.to_string(),
            description: description.map(Into::into),
            tags,
            hash: None,
            shared: false,
            toread: false,
            favourite: false,
            created_at: String::new(),
        }
        .digest()
    }
    pub fn from_matches(matches: &ArgMatches) -> Link {
        let tags = matches
            .values_of("tags")
            .map(|t| t.map(String::from).collect::<Vec<String>>());

        Link::new(
            None,
            matches.value_of("url").unwrap_or_default(),
            matches.value_of("name").unwrap_or_default(),
            matches.value_of("description"),
            tags,
        )
    }
    pub fn digest(mut self) -> Self {
        let mut hasher = Sha1::new();

        hasher.update(self.href.as_bytes());
        hasher.update(self.name.as_bytes());
        if let Some(desc) = self.description.as_ref() {
            hasher.update(desc.as_bytes());
        }
        if let Some(tags) = self.tags.as_ref() {
            hasher.update(tags.join(",").as_bytes());
        }
        hasher.update(self.toread.to_string().as_bytes());
        hasher.update(self.shared.to_string().as_bytes());
        hasher.update(self.favourite.to_string().as_bytes());
        self.hash = Some(hasher.digest().to_string());
        self
    }
    pub fn set_id(mut self, id: Option<i64>) -> Self {
        self.id = id;
        self
    }
    pub fn set_timestamp(mut self, ts: String) -> Self {
        self.created_at = ts;
        self
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
    /// Return latest version that links have been stored with for given [`User`].
    ///
    /// If user has no links yet, returns 0 as an initial version.
    fn get_latest_version(&self, user: &User) -> DBResult<Version> {
        let offset = self.get_connection().query_row(
            "SELECT ifnull(max(version), 0) FROM links WHERE user_id = ?1",
            params![user.id],
            |row| row.get::<_, i32>(0),
        )?;
        Ok(Version::new(offset))
    }
    fn store_link(
        &self,
        link: Link,
        version: Version,
        user: &User,
        txn: &Transaction,
    ) -> DBResult<(Link, Version)> {
        let offset = match version {
            Version(offset) if version.is_valid() => offset,
            _ => return Err(BadVersion),
        };
        txn.execute(
            "INSERT INTO links(href, name, description, hash, is_toread, is_shared, is_favourite, user_id, version) \
            VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
            ON CONFLICT(path(href), user_id) \
            DO UPDATE SET href = ?1, name = ?2, description = ?3, hash = ?4, is_toread = ?5, is_shared = ?6, is_favourite = ?7, \
                          version = ?9, updated_at = CURRENT_TIMESTAMP",
            params![link.href, link.name, link.description, link.hash, link.toread, link.shared, link.favourite, user.id, offset],
        )?;
        let meta: (i64, String) = txn
            .query_row(
                "SELECT id, datetime(created_at) FROM links WHERE href = ?1 AND user_id = ?2",
                params![link.href, user.id],
                |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())),
            )
            .unwrap();

        // remove connections with tags assigned to link (if it already exists)
        txn.execute("DELETE FROM links_tags WHERE link_id = ?1", params![meta.0])?;

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
                params![meta.0, Rc::new(values), user.id],
            )?;
        }
        Ok((link.set_id(Some(meta.0)).set_timestamp(meta.1), version))
    }
    pub fn add_link(&self, auth: &Option<Authentication>, link: Link) -> DBResult<Version> {
        let user = self.authenticate_user(auth)?;
        self.add_links(auth, vec![link], self.get_latest_version(&user)?)
    }
    pub fn add_links(
        &self,
        auth: &Option<Authentication>,
        links: Vec<Link>,
        version: Version,
    ) -> DBResult<Version> {
        assert!(version.is_valid());

        let user = self.authenticate_user(auth)?;
        let mut conn = self.get_connection();

        // remove all the links from request which are already
        // stored with newer version. first-write wins.

        let hrefs = Rc::<Vec<_>>::new(
            links
                .iter()
                .map(|l| path(&l.href.to_lowercase()))
                .map(SqlValue::from)
                .collect(),
        );
        let conflicting = Query::new_with_initial("SELECT lower(path(href)) FROM links WHERE")
            .concat_with_param("user_id = :user_id", (":user_id", &user.id))
            .concat_with_param("AND version >= :version", (":version", &version.offset()))
            .concat_with_param(
                "AND lower(path(href)) IN rarray(:hrefs)",
                (":hrefs", &hrefs),
            )
            .fetch_as(self.get_connection(), |row| row.get_unwrap::<_, String>(0))?;

        let mut ver = self.get_latest_version(&user)?.bump();
        let txn = conn.transaction().unwrap();
        for link in links {
            if !conflicting
                .iter()
                .any(|v| *v == path(link.href.to_lowercase().as_str()))
            {
                ver = self.store_link(link, ver, &user, &txn)?.1;
            }
        }
        txn.commit()?;
        Ok(ver)
    }
    pub fn import_links(&self, auth: &Option<Authentication>, links: Vec<Link>) -> DBResult<u32> {
        let user = self.authenticate_user(auth)?;
        let mut conn = self.get_connection();
        let mut ver = self.get_latest_version(&user)?.bump();
        let txn = conn.transaction().unwrap();

        let mut imported: u32 = 0;
        for link in links {
            let (created_link, version) = self.store_link(link, ver, &user, &txn)?;
            imported += 1;
            ver = version;
            println!("+ {}", created_link.href)
        }
        txn.commit()?;
        Ok(imported)
    }
    pub fn find_links(
        &self,
        auth: &Option<Authentication>,
        pattern: Link,
        lookup_type: DBLookupType,
        version: Version,
        limit: Option<u16>,
    ) -> DBResult<(Vec<Link>, Version)> {
        let user = self.authenticate_user(auth)?;
        let mut query = Query::new_with_initial(
            "SELECT l.id, href, name, description, group_concat(tag) AS tagz, is_toread, is_shared, is_favourite, datetime(l.created_at) \
             FROM links l \
             LEFT JOIN links_tags lt ON l.id = lt.link_id \
             LEFT JOIN tags t ON lt.tag_id = t.id WHERE",
        );

        let tags = pattern.tags.to_owned().unwrap_or_default();
        let path = path(pattern.href.as_str());
        let name = Query::patternize(&pattern.name);
        let limit = limit.unwrap_or(0);
        let offset = version.offset();

        // Apply versioning - return only these records which have version greater or equal
        // to provided one. version = -1 means that all records should be returned (except
        // from deleted ones for performance reason).

        if version.is_valid() {
            query.concat_with_param("version >= :version AND", (":version", &offset));
        } else {
            query.concat("deleted_at IS NULL AND");
        }

        // Searching by name and description is equivalent. Also, when href was not not explicitly
        // provided it's equivalent to name. This is to easily find a link by either a name/description
        // or some part of url.

        if !name.is_empty() {
            if path.is_empty() {
                query.concat_with_param(
                    "(name LIKE :name OR href LIKE :name OR description LIKE :name) AND",
                    (":name", &name),
                );
            } else {
                query.concat_with_param(
                    "(name LIKE :name OR description LIKE :name) AND",
                    (":name", &name),
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

        // 2 special flags to be handled here: toread and favourite:
        if pattern.toread {
            query.concat("l.is_toread = TRUE AND");
        }
        if pattern.favourite {
            query.concat("l.is_favourite = TRUE AND");
        }
        query.concat_with_param(
            "(l.user_id = :id OR l.is_shared) GROUP BY l.id",
            (":id", &user.id),
        );

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

        let optional_ptr = Rc::<Vec<_>>::new(optional.into_iter().map(SqlValue::from).collect());
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
        query.concat("ORDER BY l.created_at DESC, l.is_favourite DESC");

        // Finally the limit. It's not the best idea to return all the links if no constraints
        // were provided. Let's limit result up to 10 links by default.

        if limit > 0 {
            query.concat_with_param("LIMIT :limit", (":limit", &limit));
        }
        Ok((
            query.fetch(self.get_connection())?,
            self.get_latest_version(&user)?,
        ))
    }
    pub fn get_href(&self, auth: &Option<Authentication>, link_id: i64) -> DBResult<String> {
        let user = self.authenticate_user(auth)?;
        let href = self.get_connection().query_row(
            "SELECT href FROM links WHERE id = ?1 AND user_id = ?2",
            params![link_id, user.id],
            |row| row.get::<_, String>(0),
        )?;
        Ok(href)
    }
    pub fn get_link(&self, auth: &Option<Authentication>, href: &str) -> DBResult<Option<Link>> {
        let pattern = Link::new(None, &href, "", None, None);
        self.find_links(
            auth,
            pattern,
            DBLookupType::Exact,
            Version::unknown(),
            Some(1),
        )
        .map(|(links, _)| links.first().cloned())
    }
    pub fn del_link(&self, auth: &Option<Authentication>, href: &str) -> DBResult<Option<Link>> {
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
    pub fn read_link(&self, auth: &Option<Authentication>, href: &str) -> DBResult<Option<Link>> {
        match self.get_link(auth, &href) {
            Ok(Some(link)) => {
                self.get_connection().execute(
                    "UPDATE links SET is_toread = FALSE, read_at = CURRENT_TIMESTAMP WHERE id = ?",
                    params![link.id],
                )?;
                Ok(Some(link))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub fn find_matching_links(
        &self,
        auth: &Option<Authentication>,
        pattern: Link,
        version: Version,
        limit: Option<u16>,
    ) -> DBResult<(Vec<Link>, Version)> {
        self.find_links(auth, pattern, DBLookupType::Patterned, version, limit)
    }
    pub fn query_links<S: AsRef<str>>(
        &self,
        auth: &Option<Authentication>,
        query: S,
        version: Version,
        limit: Option<u16>,
    ) -> DBResult<(Vec<Link>, Version)> {
        let mut href = "";
        let mut desc = "";
        let mut name: Vec<&str> = Vec::new();
        let mut tags: Vec<Tag> = Vec::new();
        let mut toread = false;
        let mut shared = false;
        let mut favourite = false;

        for chunk in query.as_ref().split_whitespace() {
            let ch: Vec<_> = chunk.split(':').collect();
            if ch.len() == 2 {
                match ch[0] {
                    "tags" => {
                        let more_tags = ch[1].split(',').map(String::from).collect::<Vec<String>>();
                        tags.extend(more_tags);
                    }
                    "flags" => {
                        toread = ch[1].contains("toread");
                        shared = ch[1].contains("shared");
                        favourite = ch[1].contains("fav");
                    }
                    "href" => href = ch[1],
                    "desc" => desc = ch[1],
                    _ => name.push(chunk),
                }
            } else {
                name.push(chunk);
            }
        }
        let pattern = Link::new(
            None,
            href.trim(),
            name.join("%").trim(),
            Some(desc.trim()),
            Some(tags),
        )
        .set_toread(toread)
        .set_shared(shared)
        .set_favourite(favourite);

        self.find_matching_links(auth, pattern, version, limit)
    }
}

#[cfg(test)]
mod test_links {
    #![allow(unused_must_use)]

    use super::*;
    use crate::vault::test_db::{auth, vault};
    use rstest::*;

    const QUERY_EMPTY: &'static str = "";

    #[rstest]
    fn test_initial_query_links(vault: &Vault, auth: Option<Authentication>) {
        let (links, version) = vault
            .query_links(&auth, QUERY_EMPTY, Version::unknown(), None)
            .unwrap();

        assert_eq!(0, version.offset());
        assert_eq!(true, links.is_empty());
    }

    #[rstest]
    fn test_version_bump_up(vault: &Vault, auth: Option<Authentication>) {
        vault.add_link(
            &auth,
            Link::new(None, "http://foo.boo.bazz", "foo", None, None),
        );
        let (links, version) = vault
            .query_links(&auth, QUERY_EMPTY, Version::unknown(), None)
            .unwrap();

        assert_eq!(1, version.offset());
        assert_eq!(1, links.len());
    }

    #[rstest]
    fn test_returns_links_in_version(vault: &Vault, auth: Option<Authentication>) {
        // at version 0 now
        vault.add_link(
            &auth,
            Link::new(None, "http://foo.boo.bazz", "foo", None, None),
        );
        // at version 1 now
        vault.add_link(&auth, Link::new(None, "http://moo.boo", "moo", None, None));
        // at version 2 now
        let (links, version) = vault
            .query_links(&auth, QUERY_EMPTY, Version::new(2), None)
            .unwrap();

        assert_eq!(2, version.offset());
        assert_eq!(1, links.len());
        assert_eq!("moo", links.first().unwrap().name)
    }

    #[rstest]
    fn test_rejects_conflicted_links_with_the_same_version(
        vault: &Vault,
        auth: Option<Authentication>,
    ) {
        let links_1 = vec![
            Link::new(None, "http://foo.boo.bazz", "foo", None, None),
            Link::new(None, "http://moo.boo", "moo", None, None),
        ];
        let links_2 = vec![
            Link::new(None, "http://foo.boo.bazz", "foo modified", None, None),
            Link::new(None, "http://moo.io", "ioo", None, None),
        ];

        vault.add_links(&auth, links_1, Version::new(1));
        vault.add_links(&auth, links_2, Version::new(1));

        let (links, version) = vault
            .query_links(&auth, QUERY_EMPTY, Version::unknown(), None)
            .unwrap();

        let rejected = links.iter().filter(|l| l.name == "foo modified").collect::<Vec<_>>();
        assert_eq!(true, rejected.is_empty());
        assert_eq!(2, version.offset());
        assert_eq!(3, links.len());
    }
}
