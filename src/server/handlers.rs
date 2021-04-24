#![allow(warnings)]

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

#[derive(Serialize, Clone, Debug)]
struct LinksReponse {
    links: Vec<Link>,
    version: i64,
}

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
        .and_then(|header| header.split_whitespace().last());
    let auth = Authentication::from_token(token);
    let limit = request
        .get_param("limit")
        .and_then(|v| v.parse::<u16>().ok());
    let version = request
        .get_param("version")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(-1);
    let resp = router!(request,
        (POST) (/auth) => {
            match post_input!(request, {login: String, password: String}) {
                Ok(t) => match vault.user_info(&Authentication::from_credentials(t.login, t.password)) {
                    Ok(user_info) => content_encoding::apply(request, jsonize(user_info)),
                    Err(e) => {
                        err_response(e)
                    }
                }
                Err(e) => {
                    let json = try_or_400::ErrJson::from_err(&e);
                    Response::json(&json).with_status_code(400)
                }
            }
        },
        (GET) (/tags) => {
            let pattern = request.get_param("name");
            let exclude = request.get_param("exclude")
                .map(|e| e.split(',').map(|v| v.trim().to_string()).collect());

            match vault.recent_tags(&auth, pattern.as_deref(), exclude, limit) {
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
        (POST) (/queries) => {
            match post_input!(request, {name: String, query: String}) {
                Ok(t) => {
                    match vault.store_query(&auth, t.name, t.query) {
                        Ok(_) => Response::empty_204(),
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
        (DELETE) (/queries/{id: i64}) => {
            let result = vault.del_query(&auth, id);
            match result {
                Ok(_) =>  Response::empty_204(),
                Err(e) => {
                    error!("{:?}", e);
                    err_response(e)
                }
            }
        },
        (GET) (/queries) => {
            let lookup = lookup_type(request);
            match vault.find_queries(&auth, request.get_param("q").as_deref(), lookup) {
                Ok(queries) => content_encoding::apply(request, jsonize(queries)),
                Err(e) => {
                    error!("{:?}", e);
                    err_response(e)
                }
            }
        },
        (GET) (/links) => {
            let query = request.get_param("q").unwrap_or_default();
            let result = match lookup_type(request) {
                DBLookupType::Patterned => vault.query_links(&auth, query, version, limit),
                DBLookupType::Exact => {
                    let pattern = Link::new(None, query.as_str(), "", None, None);
                    vault.find_links(&auth, pattern, DBLookupType::Exact, version, limit)
                }
            };
            match result {
                Ok((links, version)) => content_encoding::apply(request, jsonize(LinksReponse{links, version})),
                Err(e) => {
                    error!("{:?}", e);
                    err_response(e)
                }
            }
        },
        (POST) (/links) => {
            match post_input!(request, {version: i64, href: String, name: String, description: String, tags: String, flags: String}) {
                Ok(t) => {
                    let tags: Vec<_> = t.tags.split(',')
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect();
                    let desc = t.description.trim();
                    let link = Link::new(
                        None,
                        &t.href,
                        &t.name,
                        if desc.is_empty() { None } else { Some(desc) },
                        if tags.is_empty() { None } else { Some(tags) }
                    )
                    .set_toread(t.flags.contains("toread"))
                    .set_shared(t.flags.contains("shared"))
                    .set_favourite(t.flags.contains("favourite"));
                    let result = vault.add_link(&auth, link, Some(t.version));
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
        (DELETE) (/links/{id: i64}) => {
            match vault.get_href(&auth, id) {
                Ok(href) => {
                    let result = vault.del_link(&auth, &href);
                    match result {
                        Ok(_) =>  Response::empty_204(),
                        Err(e) => {
                            error!("{:?}", e);
                            err_response(e)
                        }
                    }
                }
                _ => Response::empty_404()
            }
        },
        (POST) (/links/{id: i64}/read) => {
            match vault.get_href(&auth, id) {
                Ok(href) => {
                    let result = vault.read_link(&auth, &href);
                    match result {
                        Ok(_) =>  Response::empty_204(),
                        Err(e) => {
                            error!("{:?}", e);
                            err_response(e)
                        }
                    }
                }
                _ => Response::empty_404()
            }
        },
        (GET) (/search) => {
            let query = request.get_param("q").unwrap_or_default();
            let is_stored_query = query.starts_with('@');
            let fetch_links = |q| {
                match vault.query_links(&auth, q, version, limit) {
                    Ok(links) => content_encoding::apply(request, jsonize(links)),
                    Err(e) => err_response(e)
                }
            };
            if is_stored_query {
                let chunks: Vec<&str> = query.splitn(2, '/').collect();
                let is_exact = chunks.len() == 2;
                let lookup = if is_exact {
                    DBLookupType::Exact
                } else {
                    DBLookupType::Patterned
                };
                match vault.find_queries(&auth, chunks.first().unwrap().strip_prefix('@'), lookup) {
                    Ok(queries) => {
                        if !queries.is_empty() && is_exact {
                            let stored = queries.get(0).map(|q| q.query.clone()).unwrap();
                            let query = chunks.get(1).unwrap();
                            fetch_links(format!("{} {}", stored, query))
                        } else {
                            content_encoding::apply(request, jsonize(queries))
                        }
                    },
                    Err(e) => err_response(e)
                }
            } else { fetch_links(query) }
        },
        _ => {
           Response::empty_404()
        }
    );
    Ok(resp)
}
