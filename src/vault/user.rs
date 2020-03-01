use crate::db::DBError::{BadPassword, UnknownUser};
use crate::db::{DBResult, DBSeachType, Query};
use crate::utils::{confirm, password};
use crate::vault::auth::Authentication;
use crate::vault::vault::Vault;

use crate::db::DBSeachType::{Exact, Patterned};
use bcrypt::hash;
use rusqlite::params;
use std::fmt;
use std::iter::FromIterator;

#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub login: String,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.login)
    }
}

impl User {
    pub fn new(id: i64, login: &str) -> Self {
        User {
            id,
            login: login.to_string().to_ascii_lowercase(),
        }
    }
}

impl Vault {
    fn find_users(&self, pattern: Option<&str>, search: DBSeachType) -> DBResult<Vec<(User, u32)>> {
        let login = pattern.map_or(None, |v| match search {
            Exact => Some(v.to_string()),
            Patterned => Query::patternize(v),
        });
        let mut query = Query::new_with_initial(
            "SELECT u.id, login, count(l.id) FROM users u \
            LEFT JOIN links l ON l.user_id = u.id",
        );
        if login.is_some() {
            query.concat_with_param("WHERE login LIKE :login", (":login", &login));
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
    pub fn del_user(&self, login: Option<&str>) -> DBResult<Option<User>> {
        match self.find_users(login, DBSeachType::Exact) {
            Ok(users) => {
                if let Some((u, c)) = users.first() {
                    if *c == 0
                        || confirm(format!("User has {} links stored. Proceed?", *c).as_ref())
                    {
                        self.connection
                            .execute("DELETE FROM users WHERE id = ?1", params![u.id])?;
                        Ok(Some(u.clone()))
                    } else {
                        // user has been found but action is cancelled
                        Ok(None)
                    }
                } else {
                    // user not found in db
                    Err(UnknownUser)
                }
            }
            Err(e) => Err(e),
        }
    }
    pub fn passwd_user(&self, login: Option<&str>) -> DBResult<User> {
        match self.find_users(login, DBSeachType::Exact) {
            Ok(users) => {
                if let Some((u, _c)) = users.first() {
                    let pass = password(None, Some("New password"));
                    let hashed = hash(pass, 10).expect("Couldn't hash a password for some reason");
                    self.connection.execute(
                        "UPDATE users SET password=?1 WHERE id=?2",
                        params![hashed, u.id],
                    )?;
                    Ok(u.clone())
                } else {
                    Err(UnknownUser)
                }
            }
            Err(e) => Err(e),
        }
    }
    pub fn match_users(&self, pattern: Option<&str>) -> DBResult<Vec<(User, u32)>> {
        self.find_users(pattern, DBSeachType::Patterned)
    }
}
