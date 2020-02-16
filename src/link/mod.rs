use crate::utils::digest;
use std::fmt;

#[derive(Debug)]
pub struct Link<'a> {
    pub url: String,
    pub description: String,
    pub tags: &'a Vec<String>,
    pub hash: String,
}

impl fmt::Display for Link<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Link => {}\nTags => {}\n--\n{}",
            self.url,
            self.tags.join(","),
            self.description,
        )
    }
}

impl<'a> Link<'a> {
    pub fn new(url: &'a str, description: &'a str, tags: &'a Vec<String>) -> Link<'a> {
        Link {
            url: url.to_string(),
            description: description.to_string(),
            tags,
            hash: digest(url, description, tags),
        }
    }
}
