use rand::Rng;
use rpassword::read_password;
use sha1::Sha1;
use std::fs::File;
use std::io::{stdout, BufReader, Read, Write as IoWrite};

const KEY_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789";

pub fn generate_key(len: u8) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0, KEY_CHARSET.len());
            KEY_CHARSET[idx] as char
        })
        .collect()
}

pub fn digest(url: &str, notes: &Option<&str>, tags: &Option<Vec<String>>) -> String {
    let mut hasher = Sha1::new();

    hasher.update(url.as_bytes());
    if let Some(notes) = notes {
        hasher.update(notes.as_bytes());
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
        Err(_err) => 0,
    };
    contents
}

pub fn confirm(message: &str) -> bool {
    let mut input = String::new();
    print!("{} (y/N) : ", message);
    stdout().flush().unwrap();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Input expected.");
    input.is_empty() || input.trim() == "y"
}

pub fn truncate(input: &str, len: i16) -> &str {
    let mut it = input.chars();
    let mut byte_end = 0;
    let mut char_pos = 0;
    if len >= 0 {
        loop {
            if char_pos == len {
                break;
            }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_end += c.len_utf8();
            } else {
                break;
            }
        }
    }
    &input[..byte_end]
}

pub fn path(url: String) -> String {
    let parts = url.splitn(2, "://").collect::<Vec<_>>();
    parts.last().map_or(String::default(), |v| v.to_string())
}
