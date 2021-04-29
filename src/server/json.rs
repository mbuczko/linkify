use miniserde::{Deserialize, Serialize};
use rouille::{Request, Response, ResponseBody};
use std::error::Error;
use std::io::Error as IoError;
use std::io::Read;
use std::fmt;

/// Error that can happen when parsing the JSON input.
#[derive(Debug)]
pub enum JsonError {
    /// Can't parse the body of the request because it was already extracted.
    BodyAlreadyExtracted,

    /// Wrong content type.
    WrongContentType,

    /// Could not read the body from the request. Also happens if the body is not valid UTF-8.
    IoError(IoError),

    /// Error while parsing.
    ParseError,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "JsonError({:?})", self)
    }
}


impl From<IoError> for JsonError {
    fn from(err: IoError) -> JsonError {
        JsonError::IoError(err)
    }
}

impl Error for JsonError {}

pub fn json_output<T: Serialize>(result: T) -> Response {
    let json = miniserde::json::to_string(&result);
    Response {
        status_code: 200,
        headers: vec![("Content-Type".into(), "application/json".into())],
        data: ResponseBody::from_string(json),
        upgrade: None,
    }
}

pub fn json_input<O>(request: &Request) -> Result<O, JsonError> where O: Deserialize {
    if let Some(header) = request.header("Content-Type") {
        if !header.starts_with("application/json") {
            return Err(JsonError::WrongContentType);
        }
    } else {
        return Err(JsonError::WrongContentType);
    }
    if let Some(mut b) = request.data() {
        let mut buffer = String::new();
        b.read_to_string(&mut buffer)?;

        miniserde::json::from_str(&buffer).map_err(|_| JsonError::ParseError)
    } else {
        Err(JsonError::BodyAlreadyExtracted)
    }
}
