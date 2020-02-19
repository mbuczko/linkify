use rpassword::read_password;
use sha1::Sha1;
use std::fmt::Write;
use std::io::{stdout, Write as IoWrite};

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

pub fn password(password: Option<&str>) -> String {
    match password {
        Some(p) => p.to_string(),
        _ => {
            print!("Password: ");
            stdout().flush().unwrap();
            read_password().expect("Password not provided")
        }
    }
}

pub fn patternize<W: Write>(f: &mut W, s: &str) {
    f.write_fmt(format_args!("%{}%", s.to_ascii_lowercase()))
        .unwrap();
}
