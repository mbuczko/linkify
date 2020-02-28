use crate::db::DBError::{BadPassword, Unauthenticated, UnknownUser};
use crate::link::Link;
use crate::query::Query;
use crate::user::{Authentication, User};

use crate::utils::password;
use bcrypt::{hash, verify};
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
    Unauthenticated,
    UnknownUser,
    BadPassword,
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

        // keep only those which haven't been applied yet
        migrations.retain(|m| base_version.is_empty() || m.version.gt(&base_version));

        // ...and compose final transaction
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
                    Ok((
                        row.get(0)?,
                        Version::parse(&row.get::<_, String>(1)?).unwrap(),
                    ))
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
    fn store_link(&mut self, link: &Link, user: &User) -> DBResult<i64> {
        let txn = self.connection.transaction().unwrap();
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
        txn.commit().and(Ok(id)).map_err(Into::into)
    }
    pub fn add_link(&mut self, link: &Link, auth: &Option<Authentication>) -> DBResult<i64> {
        match self.authenticate_user(auth) {
            Ok(u) => self.store_link(link, &u),
            Err(e) => return Err(e),
        }
    }
    pub fn import_links(
        &mut self,
        links: Vec<Link>,
        auth: &Option<Authentication>,
    ) -> DBResult<u32> {
        match self.authenticate_user(auth) {
            Ok(u) => {
                let mut imported: u32 = 0;
                for link in links {
                    if self.store_link(&link, &u).is_ok() {
                        imported += 1;
                        println!("+ {}", link.href)
                    }
                }
                Ok(imported)
            }
            Err(e) => return Err(e),
        }
    }
    pub fn match_links(
        &mut self,
        link: &Link,
        auth: &Option<Authentication>,
    ) -> DBResult<Vec<Link>> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let tags = link.tags.to_owned().unwrap_or_default();
        let ptr = Rc::new(tags.into_iter().map(SqlValue::from).collect());
        let href = Query::like(&link.href).unwrap_or_default();
        let desc = link
            .description
            .as_ref()
            .map_or(None, |v| Query::like(v))
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
            query.concat_with_param("lower(description) LIKE :desc AND", (":desc", &desc));
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

    pub fn add_user(&self, auth: &Option<Authentication>) -> DBResult<User> {
        match auth {
            Some(auth) => {
                let hashed =
                    hash(&auth.password, 10).expect("Couldn't hash a password for some reason");
                self.connection.execute(
                    "INSERT INTO users(login, password) VALUES(?1, ?2)",
                    params![auth.login, hashed],
                )?;
                Ok(User::new(self.connection.last_insert_rowid(), &auth.login))
            }
            _ => Err(BadPassword),
        }
    }
    pub fn passwd_user(&self, auth: &Option<Authentication>) -> DBResult<User> {
        let user = match self.authenticate_user(auth) {
            Ok(u) => u,
            Err(e) => return Err(e),
        };
        let pass = password(None, Some("New password"));
        let hashed = hash(pass, 10).expect("Couldn't hash a password for some reason");
        self.connection.execute(
            "UPDATE users SET password=?1 WHERE id=?2",
            params![hashed, user.id],
        )?;
        Ok(user)
    }
    pub fn authenticate_user(&self, auth: &Option<Authentication>) -> DBResult<User> {
        auth.as_ref().map_or(Err(Unauthenticated), |a| {
            self.connection
                .query_row(
                    "SELECT id, login, password FROM users WHERE login = ?1",
                    params![a.login],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_or(Err(UnknownUser), |user: (i64, String, String)| {
                    if verify(&a.password, &user.2).unwrap_or(false) {
                        Ok(User::new(user.0, &user.1))
                    } else {
                        Err(BadPassword)
                    }
                })
        })
    }
    pub fn match_users(&self, pattern: Option<&str>) -> DBResult<Vec<(User, u16)>> {
        let mut query = Query::new_with_initial(
            "SELECT u.id, login, count(l.id) FROM users u \
            LEFT JOIN links l ON l.user_id = u.id",
        );
        let login = pattern
            .map_or(None, |v| Query::like(v.to_ascii_lowercase().as_str()))
            .unwrap_or_default();

        if !login.is_empty() {
            query.concat_with_param("WHERE lower(login) like :login", (":login", &login));
        }
        query.concat("GROUP BY login");

        let mut stmt = self.connection.prepare(query.to_string().as_str())?;
        let rows = stmt.query_map_named(query.named_params(), |row| {
            Ok((
                User {
                    id: row.get(0).unwrap(),
                    login: row.get(1).unwrap(),
                },
                row.get_unwrap(2),
            ))
        })?;
        Result::from_iter(rows).map_err(Into::into)
    }

    //
    // vault
    //

    pub fn new(db: &str) -> Self {
        match Connection::open(db) {
            Ok(conn) => {
                array::load_module(&conn).unwrap();
                Vault { connection: conn }
            }
            _ => panic!("Cannot open connection to database or load required modules (array)"),
        }
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
