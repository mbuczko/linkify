use sha1::{Sha1};

pub fn digest(url: &str, description: &Option<&str>, tags: &Option<Vec<String>>) -> String {
    let mut hasher = Sha1::new();

    hasher.update(url.as_bytes());
    if let Some(description) = description {
        hasher.update(description.as_bytes());
    }
    if let Some(tags) = tags {
        hasher.update(tags.join(",").as_bytes());
    }
    hasher.digest().to_string()
}
