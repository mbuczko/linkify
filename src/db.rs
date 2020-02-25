use crate::link::Link;
use crate::query::Query;
use crate::user::User;
use crate::utils::patternize;
use bcrypt::hash;
use log::debug;
use rusqlite::types::Value as SqlValue;
use rusqlite::{
    params, vtab::array, Connection, Error as SqliteError, Result as SqliteResult, NO_PARAMS,
};
use rust_embed::RustEmbed;
use semver::Version;
use std::fmt;
use std::iter::FromIterator;
use std::rc::Rc;
use std::str;

#[derive(Debug)]
pub enum DBError {
    Sqlite(SqliteError),
}

type DBResult<T> = Result<T, DBError>;

#[derive(RustEmbed)]
#[folder = "resources/db/migrations/"]
struct Asset;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Migration {
    file: String,
    version: String,
    description: String,
}

#[derive(Debug)]
pub struct Vault {
    connection: Connection,
}

impl From<SqliteError> for DBError {
    fn from(err: SqliteError) -> Self {
        DBError::Sqlite(err)
    }
}

impl Migration {
    pub fn new(file: String, version: String, description: String) -> Self {
        Migration {
            file,
            version,
            description,
        }
    }
}

impl fmt::Display for Migration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} => {} ({})",
            self.file, self.version, self.description
        )
    }
}

impl Vault {
    //
    // migrations
    //

    fn build_migration(&self, base_version: String, app_semver: Version) -> Option<String> {
        let mut migrations: Vec<Migration> = Asset::iter()
            .map(|file| {
                let v: Vec<&str> = file.split("__").collect();
                Migration::new(
                    file.to_string(),
                    v[0].to_string(),
                    v[1].trim_end_matches(".sql").replace("_", " "),
                )
            })
            .collect();

        // sort migrations by versions first
        migrations.sort_by(|m1, m2| m1.version.cmp(&m2.version));

        // ...and keep only those which haven't been applied yet
        migrations.retain(|m| base_version.is_empty() || m.version.gt(&base_version));

        // compose final transaction
        let final_txn = migrations.iter().fold(String::default(), |mut txn, m| {
            let buf = Asset::get(m.file.as_ref()).unwrap();
            match str::from_utf8(&buf) {
                Ok(s) => {
                    txn.push_str(s);
                    txn.push_str(
                        format!(
                            "\n\n\
                    INSERT INTO migrations(version, description, script, app_semver) \
                    VALUES ('{version}', '{description}', '{script}', '{semver}');\n\n",
                            version = m.version,
                            description = m.description,
                            script = m.file,
                            semver = app_semver
                        )
                        .as_str(),
                    );
                    txn
                }
                _ => panic!("Non UTF8 format of migration file!"),
            }
        });
        return if final_txn.is_empty() {
            None
        } else {
            Some(format!("BEGIN TRANSACTION;\n\n{}\n\nCOMMIT;", final_txn))
        };
    }
    fn version(&self) -> DBResult<(String, Version)> {
        self.connection
            .query_row(
                "SELECT version, app_semver FROM migrations ORDER BY version DESC LIMIT 1",
                NO_PARAMS,
                |row| {
                    let ver: String = row.get(1)?;
                    Ok((row.get(0)?, Version::parse(&ver).unwrap()))
                },
            )
            .map_err(Into::into)
    }
    fn upgrade(&self, base_script_version: String, app_semver: Version) {
        if let Some(m) = self.build_migration(base_script_version, app_semver) {
            match self.connection.execute_batch(m.as_str()) {
                Ok(_) => debug!("Upgraded to {}", self.version().unwrap().0),
                _ => panic!("Couldn't update the database. Bailing out."),
            }
        }
    }

    //
    // links
    //

    pub fn add_link<'a>(&mut self, link: &'a Link) -> DBResult<&'a Link> {
        let txn = self.connection.transaction().unwrap();
        txn.execute(
            "INSERT INTO links(url, description, hash) VALUES(?1, ?2, ?3) \
            ON CONFLICT(url) \
            DO UPDATE SET description = ?2, hash = ?3",
            params![link.url, link.description, link.hash],
        )?;

        // note that last_insert_rowid returns 0 for already existing URLs
        let id = match txn.last_insert_rowid() {
            0 => txn
                .query_row(
                    "SELECT id FROM links WHERE url = ?1 AND user_id IS NULL",
                    params![link.url],
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
                    "INSERT INTO tags(tag, user_id) VALUES(?1, NULL) \
            ON CONFLICT(tag, user_id) \
            DO UPDATE SET used_at = CURRENT_TIMESTAMP",
                    params![tag],
                )?;
                values.push(SqlValue::from(tag.to_string()));
            }
            txn.execute(
                "INSERT INTO links_tags(link_id, tag_id) \
        SELECT ?1, id FROM tags WHERE tag IN rarray(?2) AND user_id IS NULL",
                params![id, Rc::new(values)],
            )?;
        }
        txn.commit().and(Ok(link)).map_err(Into::into)
    }
    pub fn new(db: &str) -> Self {
        match Connection::open(db) {
            Ok(conn) => {
                array::load_module(&conn).unwrap();
                Vault { connection: conn }
            }
            _ => panic!("Cannot open connection to database or load required modules (array)"),
        }
    }
    pub fn match_links(&mut self, link: &Link) -> DBResult<Vec<Link>> {
        let tags = link.tags.to_owned().unwrap_or_default();
        let ptr = Rc::new(tags.into_iter().map(SqlValue::from).collect());
        let url = Query::like(&link.url).unwrap_or_default();
        let desc = link
            .description
            .as_ref()
            .map_or(None, |v| Query::like(v))
            .unwrap_or_default();

        let mut query = Query::new_with_initial(
            "SELECT url, description, group_concat(tag) FROM links l \
        LEFT JOIN links_tags lt ON l.id = lt.link_id \
        LEFT JOIN tags t ON lt.tag_id = t.id \
        WHERE",
        );
        if !url.is_empty() {
            query.concat_with_named_param("url LIKE :url AND", (":url", &url));
        }
        if !desc.is_empty() {
            query.concat_with_named_param("lower(description) LIKE :desc AND", (":desc", &desc));
        }
        query.concat("1=1 GROUP BY l.id");
        if link.tags.is_some() {
            query.concat_with_named_param(
                "HAVING l.id IN \
            (SELECT link_id FROM links_tags lt2 \
            JOIN tags t2 ON lt2.tag_id = t2.id AND t2.tag IN rarray(:tags))",
                (":tags", &ptr),
            );
        }
        query.concat("ORDER BY l.created_at DESC");

        let mut stmt = self.connection.prepare(query.to_string().as_str())?;
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

    //
    // users
    //

    pub fn add_user(&self, login: &str, password: String) -> DBResult<User> {
        let hashed = hash(password, 10).expect("Couldn't hash a password for some reason");
        self.connection.execute(
            "INSERT INTO users(login, password) VALUES(?1, ?2)",
            params![login, hashed],
        )?;
        Ok(User::from(login))
    }
    pub fn passwd_user(&self, login: &str, new_password: String) -> DBResult<User> {
        let hashed = hash(new_password, 10).expect("Couldn't hash a password for some reason");
        self.connection.execute(
            "UPDATE users SET password=?1 WHERE login=?2",
            params![login, hashed],
        )?;
        Ok(User::from(login))
    }
    pub fn match_users(&self, pattern: Option<&str>) -> DBResult<Vec<(User, u16)>> {
        let mut user_pattern = String::with_capacity(32);
        let mut params = Vec::new();
        let mut query = vec![
            "SELECT login, count(l.id) FROM users u \
            LEFT JOIN links l ON l.user_id = u.id",
        ];
        if let Some(pat) = pattern {
            patternize(&mut user_pattern, pat);
            query.push("WHERE lower(login) like ?1");
            params.push(user_pattern.to_ascii_lowercase());
        }
        query.push("GROUP BY login");

        let mut stmt = self.connection.prepare(query.join(" ").as_str())?;
        let rows = stmt.query_map(&params, |row| {
            Ok((
                User {
                    login: row.get(0).unwrap(),
                },
                row.get_unwrap(1),
            ))
        })?;
        Result::from_iter(rows).map_err(Into::into)
    }
}

pub fn init_vault(db: &str, app_semver: Version) -> SqliteResult<Vault> {
    let vault = Vault::new(db);
    let (last_script_version, last_app_version) = match vault.version() {
        Ok((lsv, lav)) => (lsv, lav),
        Err(_) => (String::default(), Version::parse("0.0.0").unwrap()),
    };

    if last_app_version > app_semver {
        panic!(
            "Your app version {} is too old, minimal required version is: {}",
            app_semver, last_app_version
        )
    } else if last_app_version < app_semver {
        debug!("Upgrading data version to {}", app_semver);
        vault.upgrade(last_script_version, app_semver);
    }
    Ok(vault)
}
