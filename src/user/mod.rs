use std::fmt;

#[derive(Debug)]
pub struct User {
    pub login: String,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.login)
    }
}

impl User {
    pub fn from(id: &str) -> User {
        User {
            login: id.to_string(),
        }
    }
}
