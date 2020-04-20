use crate::db::DBError::{Unauthenticated, UnknownUser};
use crate::db::{DBError, DBLookupType};
use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::Vault;

use failure::Error;
use log::error;
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

fn lookup_type(request: &Request) -> DBLookupType {
    request
        .get_param("exact")
        .map_or(DBLookupType::Patterned, |v| {
            if v == "true" {
                DBLookupType::Exact
            } else {
                DBLookupType::Patterned
            }
        })
}

pub fn handler(request: &Request, vault: &Vault) -> HandlerResult {
    let token = request
        .header("authorization")
        .map_or(None, |header| header.split_whitespace().last());
    let limit = request
        .get_param("limit")
        .and_then(|v| v.parse::<u16>().ok());
    let resp = router!(request,
        (POST) (/searches) => {
            match post_input!(request, {name: String, query: String}) {
                Ok(t) => {
                    match vault.store_search(Authentication::from_token(token), t.name, t.query) {
                        Ok(id) => Response {
                            status_code: 200,
                            headers: vec![("Location".into(), format!("/searches/{}", id).into())],
                            data: ResponseBody::empty(),
                            upgrade: None,
                        },
                        Err(e) => {
                            error!("{:?}", e);
                            err_response(e)
                        }
                    }
                }
                Err(e) => {
                    let json = try_or_400::ErrJson::from_err(&e);
                    Response::json(&json).with_status_code(400)
                }
            }
        },
        (GET) (/searches) => {
            match vault.find_searches(
                Authentication::from_token(token),
                request.get_param("name").as_deref(),
                lookup_type(request)
            ) {
                Ok(searches) => content_encoding::apply(request, jsonize(searches)),
                Err(e) => err_response(e)
            }
        },
        (GET) (/tags) => {
            let tag_name = request.get_param("name");
            match vault.recent_tags(Authentication::from_token(token), tag_name.as_deref(), limit) {
                Ok(tags) => {
                    let mut result = HashMap::new();
                    result.insert("tags", tags);
                    content_encoding::apply(request, jsonize(result))
                }
                Err(e) => {
                    error!("{:?}", e);
                    err_response(e)
                }
            }
        },
        (GET) (/links) => {
            let authentication = Authentication::from_token(token);
            let query = request.get_param("q");
            let href = request.get_param("href").unwrap_or_default();
            let result = if query.is_some() {
                vault.query(authentication, query.unwrap(), limit)
            } else {
                vault.find_links(authentication, Link::from(href.as_str()), DBLookupType::Exact, limit)
            };
            match result {
                Ok(links) => content_encoding::apply(request, jsonize(links)),
                Err(e) => {
                    error!("{:?}", e);
                    err_response(e)
                }
            }
        },
        (POST) (/links) => {
            match post_input!(request, {href: String, title: String, notes: String, tags: String, flags: String}) {
                Ok(t) => {
                    debug!("{:?}", t);
                    let tags: Vec<_> = t.tags.split(',').into_iter()
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect();
                    let notes = t.notes.trim();
                    let link = Link::new(
                        &t.href,
                        &t.title,
                        if notes.is_empty() { None } else { Some(notes) },
                        if tags.is_empty() { None } else { Some(tags) }
                    );
                    let result = vault.add_link(Authentication::from_token(token), link);
                    match result {
                        Ok(_) =>  Response::empty_204(),
                        Err(e) => {
                            error!("{:?}", e);
                            err_response(e)
                        }
                    }
                }
                Err(e) => {
                    let json = try_or_400::ErrJson::from_err(&e);
                    Response::json(&json).with_status_code(400)
                }
            }
        },
        (DELETE) (/links) => {
            match post_input!(request, {url: String}) {
                Ok(t) => {
                    let result = vault.del_link(Authentication::from_token(token), Link::from(t.url.as_str()));
                    match result {
                        Ok(_) =>  Response::empty_204(),
                        Err(e) => {
                            error!("{:?}", e);
                            err_response(e)
                        }
                    }
                }
                Err(e) => {
                    let json = try_or_400::ErrJson::from_err(&e);
                    Response::json(&json).with_status_code(400)
                }
            }
        },
        _ => {
           Response::empty_404()
        }
    );
    Ok(resp)
}
