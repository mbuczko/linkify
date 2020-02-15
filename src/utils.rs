use sha1::{Sha1};

pub fn digest(url: &str, description: &str, tags: &Vec<&str>) -> String {
    let mut hasher = Sha1::new();

    hasher.update(url.as_bytes());
    hasher.update(description.as_bytes());
    hasher.update(tags.join(",").as_bytes());
    hasher.digest().to_string()
}
