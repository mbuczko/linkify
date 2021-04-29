use miniserde::{Deserialize};

#[derive(Deserialize, Debug)]
pub struct AuthRequest {
    pub login: String,
    pub password: String

}

#[derive(Deserialize, Debug)]
pub struct QueryRequest {
    pub name: String,
    pub query: String

}

#[derive(Deserialize, Debug)]
pub struct LinksRequest {
    pub version: i32,
    pub links: Vec<LinkPostData>,
}

#[derive(Deserialize, Debug)]
pub struct LinkPostData {
    pub href: String,
    pub name: String,
    pub description: String,
    pub tags: Option<String>,
    pub flags: Option<String>
}
