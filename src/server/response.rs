use crate::db::DBError;
use crate::db::DBError::{Unauthenticated, UnknownUser};
use crate::vault::link::Link;

use log::error;
use miniserde::Serialize;
use rouille::{Response, ResponseBody};

#[derive(Serialize, Clone, Debug)]
pub struct LinksResponse {
    pub version: i32,
    pub links: Vec<Link>,
}


pub fn empty_40x(code: u16) -> Response {
    Response {
        status_code: code,
        headers: vec![],
        data: ResponseBody::empty(),
        upgrade: None,
    }
}

pub fn err_response(err: DBError) -> Response {
    error!("{:?}", err);
    match err {
        UnknownUser => empty_40x(403),
        Unauthenticated => empty_40x(401),
        e => Response::text(e.to_string()).with_status_code(400),
    }
}
