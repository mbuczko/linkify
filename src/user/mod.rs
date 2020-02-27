use crate::utils::password;
use clap::ArgMatches;
use std::fmt;

#[derive(Debug)]
pub struct User {
    pub login: String,
}

#[derive(Debug)]
pub struct Authentication {
    pub login: String,
    pub password: String,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.login)
    }
}

impl User {
    pub fn new(login: &str) -> Self {
        User {
            login: login.to_string(),
        }
    }
}

impl Authentication {
    pub fn from(matches: &ArgMatches) -> Option<Self> {
        let login = matches.value_of("user");
        if login.is_some() {
            Some(Authentication {
                login: login.unwrap().to_string(),
                password: password(matches.value_of("password")),
            })
        } else {
            None
        }
    }
}
