use crate::db::DBError::{Unauthenticated, UnknownUser};
use crate::db::{DBError, DBLookupType};
use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::Vault;

use failure::Error;
use miniserde::{json, Serialize};
use rouille::content_encoding;
use rouille::{post_input, router, try_or_400, Request, Response, ResponseBody};
use std::collections::HashMap;

pub type HandlerResult = Result<Response, Error>;

fn jsonize<T: Serialize>(result: T) -> Response {
    let json = json::to_string(&result);
    Response {
        status_code: 200,
        headers: vec![("Content-Type".into(), "application/json".into())],
        data: ResponseBody::from_string(json),
        upgrade: None,
    }
}

fn empty_40x(code: u16) -> Response {
    Response {
        status_code: code,
        headers: vec![],
        data: ResponseBody::empty(),
        upgrade: None,
    }
}

fn err_response(err: DBError) -> Response {
    match err {
        UnknownUser => empty_40x(403),
        Unauthenticated => empty_40x(401),
        e => Response::text(e.to_string()).with_status_code(400),
    }
}

pub fn handler(request: &Request, vault: &Vault) -> HandlerResult {
    let mut link = Link::default();
    let token = request
        .header("authorization")
        .map_or(None, |header| header.split_whitespace().last());
    let authentication = Authentication::from_token(token);
    let limit = request
        .get_param("limit")
        .and_then(|v| v.parse::<u16>().ok());
    let resp = router!(request,
        (POST) (/searches) => {
            match post_input!(request, {name: String, query: String}) {
                Ok(t) => {
                    match vault.store_search(&authentication, t.name, t.query) {
                        Ok(id) => Response {
                            status_code: 200,
                            headers: vec![("Location".into(), format!("/searches/{}", id).into())],
                            data: ResponseBody::empty(),
                            upgrade: None,
                        },
                        Err(e) => err_response(e)
                    }
                }
                Err(e) => {
                    let json = try_or_400::ErrJson::from_err(&e);
                    Response::json(&json).with_status_code(400)
                }
            }
        },
        (GET) (/searches) => {
            let search_name = request.get_param("name");
            let search_type = request.get_param("exact").map_or(DBLookupType::Patterned, |v| {
                if v == "true" {
                    DBLookupType::Exact
                } else {
                    DBLookupType::Patterned
                }
            });
            match vault.find_searches(&authentication, search_name.as_deref(), search_type) {
                Ok(searches) => content_encoding::apply(request, jsonize(searches)),
                Err(e) => err_response(e)
            }
        },
        (GET) (/tags) => {
            let tag_name = request.get_param("name");
            match vault.recent_tags(&authentication, tag_name.as_deref(), limit) {
                Ok(tags) => {
                    let mut result = HashMap::new();
                    result.insert("tags", tags);
                    content_encoding::apply(request, jsonize(result))
                }
                Err(e) => err_response(e)
            }
        },
        (GET) (/links) => {
            let omni = request.get_param("omni");
            let result = if omni.is_some() {
                vault.omni_search(&authentication, omni.unwrap(), limit)
            } else {
                vault.match_links(
                    &authentication,
                    link
                        .with_title(request.get_param("title"))
                        .with_tags(request.get_param("tags"))
                        .with_notes(request.get_param("notes")),
                    limit, false)
            };
            match result {
                Ok(links) => content_encoding::apply(request, jsonize(links)),
                Err(e) => err_response(e)
            }
        },
        _ => {
           Response::empty_404()
        }
    );
    Ok(resp)
}
