use crate::utils::digest;
use std::fmt;
use clap::ArgMatches;

type Tag = String;

#[derive(Debug)]
pub struct Link {
    pub url: String,
    pub description: Option<String>,
    pub tags: Option<Vec<Tag>>,
    pub hash: String,
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = vec![self.url.as_str()];
        let tags = self.tags.as_ref().map_or(None, |t| Some(t.join(" ")));

        if let Some(d) = self.description.as_ref() {
            s.push(d);
        }
        if let Some(t) = tags.as_ref() {
            s.push("--");
            s.push(t);
        }
        write!(f, "{}\n", s.join("\n"))
    }
}

impl Link {
    pub fn new(url: &str, description: Option<&str>, tags: Option<Vec<Tag>>) -> Link {
        Link {
            url: url.to_string(),
            description: description.map(Into::into),
            hash: digest(url, &description, &tags),
            tags,
        }
    }
    pub fn from(matches: &ArgMatches) -> Link {
        let tags = matches
            .values_of("tags")
            .and_then(|t| Some(t.map(String::from).collect::<Vec<String>>()));

        Link::new(
            matches.value_of("url").unwrap_or_default(),
            matches.value_of("description"),
            tags
        )
    }
}

