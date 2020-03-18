use crate::db::DBError::UnknownUser;
use crate::db::DBSeachType::{Exact, Patterned};
use crate::db::{DBResult, DBSeachType};
use crate::utils::{confirm, generate_key, password};
use crate::vault::Vault;

use crate::db::query::Query;
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

        let conn = self.get_connection();
        let mut stmt = conn.prepare(query.to_string().as_str())?;
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
    pub fn find_user(&self, login: Option<&str>) -> DBResult<(User, u32)> {
        if login.is_some() {
            let users = self.find_users(login, DBSeachType::Exact)?;
            users
                .first()
                .map_or(Err(UnknownUser), |(user, count)| Ok((user.clone(), *count)))
        } else {
            Err(UnknownUser)
        }
    }
    pub fn match_users(&self, pattern: Option<&str>) -> DBResult<Vec<(User, u32)>> {
        self.find_users(pattern, DBSeachType::Patterned)
    }
    pub fn add_user(&self, login: Option<&str>) -> DBResult<User> {
        match login {
            Some(l) => {
                let pass = password(None, Some("Initial password"));
                let hashed = hash(pass, 10).expect("Couldn't hash a password for some reason.");
                self.get_connection().execute(
                    "INSERT INTO users(login, password) VALUES(?1, ?2)",
                    params![l, hashed],
                )?;
                Ok(User::new(self.get_connection().last_insert_rowid(), &l))
            }
            _ => Err(UnknownUser),
        }
    }
    pub fn del_user(&self, login: Option<&str>) -> DBResult<(User, bool)> {
        if let Ok((u, c)) = self.find_user(login) {
            if c == 0 || confirm(format!("User {} has {} links. Proceed?", u.login, c).as_ref()) {
                self.get_connection()
                    .execute("DELETE FROM users WHERE id = ?1", params![u.id])?;
                Ok((u, true))
            } else {
                Ok((u, false))
            }
        } else {
            Err(UnknownUser)
        }
    }
    pub fn passwd_user(&self, login: Option<&str>) -> DBResult<User> {
        if let Ok((u, _count)) = self.find_user(login) {
            let pass = password(None, Some("New password"));
            let hashed = hash(pass, 10).expect("Couldn't hash a password for some reason");
            self.get_connection().execute(
                "UPDATE users SET password=?1 WHERE id=?2",
                params![hashed, u.id],
            )?;
            Ok(u)
        } else {
            Err(UnknownUser)
        }
    }
    pub fn generate_key(&self, login: Option<&str>) -> DBResult<(User, String)> {
        if let Ok((u, _count)) = self.find_user(login) {
            let key = generate_key(32);
            self.get_connection().execute(
                "UPDATE users SET api_key = ?1 WHERE id = ?2",
                params![key, u.id],
            )?;
            Ok((u, key))
        } else {
            Err(UnknownUser)
        }
    }
}
