use crate::db::DBError::UnknownUser;
use crate::db::DBLookupType::{Exact, Patterned};
use crate::db::{DBLookupType, DBResult};
use crate::utils::{confirm, password, random_string};
use crate::vault::Vault;

use crate::db::query::Query;
use bcrypt::hash;
use rusqlite::params;
use std::fmt;

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
    /// Does a user look up by name, depending on [lookup type] either exactly as provided or as a substring.
    ///
    /// [lookup type]: DBLookupType
    fn find_users(&self, pattern: &str, lookup_type: DBLookupType) -> DBResult<Vec<(User, u32)>> {
        let login = match lookup_type {
            Exact => pattern.to_owned(),
            Patterned => Query::patternize(pattern),
        };
        Query::new_with_initial(
            "SELECT u.id, login, count(l.id) FROM users u \
            LEFT JOIN links l ON l.user_id = u.id",
        )
        .concat_with_param("WHERE login LIKE :login GROUP BY login", (":login", &login))
        .fetch_as(self.get_connection(), |row| {
            (
                User {
                    id: row.get(0).unwrap(),
                    login: row.get(1).unwrap(),
                },
                row.get_unwrap(2),
            )
        })
    }
    pub fn find_user(&self, login: &str) -> DBResult<(User, u32)> {
        let users = self.find_users(login, DBLookupType::Exact)?;
        users
            .first()
            .map_or(Err(UnknownUser), |(user, count)| Ok((user.clone(), *count)))
    }
    pub fn match_users(&self, pattern: &str) -> DBResult<Vec<(User, u32)>> {
        self.find_users(pattern, DBLookupType::Patterned)
    }
    pub fn add_user(&self, login: &str, password: &str) -> DBResult<User> {
        let hashed = hash(password, 10).expect("Couldn't hash a password for some reason.");
        self.get_connection().execute(
            "INSERT INTO users(login, password) VALUES(?1, ?2)",
            params![login, hashed],
        )?;
        Ok(User::new(self.get_connection().last_insert_rowid(), login))
    }
    pub fn del_user(&self, login: &str) -> DBResult<(User, bool)> {
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
    pub fn passwd_user(&self, login: &str) -> DBResult<User> {
        if let Ok((u, _count)) = self.find_user(login) {
            let pass = password(None, Some("New password"));
            let hashed = hash(pass, 10).expect("Couldn't hash a password for some reason");
            self.get_connection().execute(
                "UPDATE users SET password = ?1 WHERE id = ?2",
                params![hashed, u.id],
            )?;
            Ok(u)
        } else {
            Err(UnknownUser)
        }
    }
    pub fn generate_key(&self, login: &str) -> DBResult<(User, String)> {
        if let Ok((u, _count)) = self.find_user(login) {
            let key = random_string(32);
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

#[cfg(test)]
mod test_user {
    use super::*;
    use crate::vault::test_db::{auth, vault};
    use crate::Authentication;
    use rstest::*;

    #[rstest]
    fn test_add_new_user(vault: &Vault, #[with("boo")] auth: Option<Authentication>) {
        let user_info = vault.user_info(&auth);
        assert_eq!("boo", user_info.unwrap().login);
    }
}
