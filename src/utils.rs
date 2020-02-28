use rpassword::read_password;
use sha1::Sha1;
use std::io::{stdout, Write as IoWrite, BufReader, Read};
use std::fs::File;

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

pub fn password(password: Option<&str>, prompt: Option<&str>) -> String {
    match password {
        Some(p) => p.to_string(),
        _ => {
            print!("{}: ", prompt.unwrap_or("Password"));
            stdout().flush().unwrap();
            read_password().expect("Password not provided")
        }
    }
}

pub fn read_file(filepath: &str) -> String {
    let file = File::open(filepath).expect("Could not open file");
    let mut buffered_reader = BufReader::new(file);
    let mut contents = String::new();
    let _number_of_bytes: usize = match buffered_reader.read_to_string(&mut contents) {
        Ok(number_of_bytes) => number_of_bytes,
        Err(_err) => 0
    };
    contents
}
