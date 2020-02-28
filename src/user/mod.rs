use crate::auth::Authentication;
use crate::vault::DBError::{BadPassword};
use crate::vault::{DBResult, Vault};
use crate::query::Query;
use crate::utils::password;

use bcrypt::{hash};
use rusqlite::params;
use std::fmt;
use std::iter::FromIterator;

#[derive(Debug)]
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
            login: login.to_string(),
        }
    }
}

impl Vault {
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
}
