use crate::db::DBLookupType;
use crate::server::json::*;
use crate::server::request::*;
use crate::server::response::*;
use crate::vault::auth::Authentication;
use crate::vault::link::{Link, Version};
use crate::vault::Vault;

use failure::Error;
use log::error;
use rouille::{content_encoding, router, try_or_400, Request, Response};
use std::collections::HashMap;

pub type HandlerResult = Result<Response, Error>;

fn lookup_type(request: &Request) -> DBLookupType {
    request
        .get_param("exact")
        .map_or(DBLookupType::Patterned, |v| {
            if v.to_lowercase() == "true" {
                DBLookupType::Exact
            } else {
                DBLookupType::Patterned
            }
        })
}

pub fn api_handler(request: &Request, vault: &Vault) -> HandlerResult {
    let token = request
        .header("authorization")
        .and_then(|header| header.split_whitespace().last());
    let auth = Authentication::from_token(token);
    let limit = request
        .get_param("limit")
        .and_then(|v| v.parse::<u16>().ok());

    // version sent in GET queries
    let version = Version::new(
        request
            .get_param("version")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(-1),
    );

    #[allow(clippy::clippy::manual_strip)]
    let resp = router!(request,
        (GET) (/version) => {
            Response::text(env!("CARGO_PKG_VERSION"))
        },
        (POST) (/auth) => {
            match json_input::<AuthRequest>(request) {
                Ok(t) => match vault.user_info(&Authentication::from_credentials(t.login, t.password)) {
                    Ok(user_info) => content_encoding::apply(request, json_output(user_info)),
                    Err(e) => err_response(e)
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
                    content_encoding::apply(request, json_output(result))
                }
                Err(e) => err_response(e)
            }
        },
        (POST) (/queries) => {
            match json_input::<QueryRequest>(request) {
                Ok(t) => {
                    match vault.store_query(&auth, t.name, t.query) {
                        Ok(_) => Response::empty_204(),
                        Err(e) => err_response(e)
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
                Err(e) => err_response(e)
            }
        },
        (GET) (/queries) => {
            let lookup = lookup_type(request);
            match vault.find_queries(&auth, request.get_param("q").as_deref(), lookup) {
                Ok(queries) => content_encoding::apply(request, json_output(queries)),
                Err(e) => err_response(e)
            }
        },
        (GET) (/links) => {
            let query = request.get_param("q").unwrap_or_default();
            let result = match lookup_type(request) {
                DBLookupType::Patterned => vault.query_links(&auth, query, version.clone(), limit),
                DBLookupType::Exact => {
                    let pattern = Link::new(None, query.as_str(), "", None, None);
                    vault.find_links(&auth, pattern, DBLookupType::Exact, version.clone(), limit)
                }
            };
            match result {
                Ok((links, version)) => content_encoding::apply(request, json_output(LinksResponse{links, version: version.offset()})),
                Err(e) => err_response(e)
            }
        },
        (POST) (/links) => {
            match json_input::<LinksRequest>(request) {
                Ok(res) => {
                    let version = Version::new(res.version);
                    let links = res.links.into_iter().map(|link| {
                        let tags: Vec<_> = link.tags.unwrap_or_default().split(',')
                            .map(|v| v.trim().to_string())
                            .filter(|v| !v.is_empty())
                            .collect();
                        let flags = link.flags.unwrap_or_default();
                        let desc = link.description.trim();
                        Link::new(
                            None,
                            &link.href,
                            &link.name,
                            if desc.is_empty() { None } else { Some(desc) },
                            if tags.is_empty() { None } else { Some(tags) }
                        )
                            .set_toread(flags.contains("toread"))
                            .set_shared(flags.contains("shared"))
                            .set_favourite(flags.contains("favourite"))
                    }).collect();
                    vault.add_links(&auth, links, version)?;
                    Response::empty_204()
                }
                Err(e) => {
                    error!("{:?}", e);
                    Response::empty_400()
                }
            }
        },
        (DELETE) (/links/{id: i64}) => {
            match vault.get_href(&auth, id) {
                Ok(href) => {
                    let result = vault.del_link(&auth, &href);
                    match result {
                        Ok(_) =>  Response::empty_204(),
                        Err(e) => err_response(e)
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
                        Err(e) => err_response(e)
                    }
                }
                _ => Response::empty_404()
            }
        },
        (GET) (/search) => {
            let query = request.get_param("q").unwrap_or_default();
            let is_stored_query = query.starts_with('@');
            let fetch_links = |q, v| {
                match vault.query_links(&auth, q, v, limit) {
                    Ok((links, _)) => content_encoding::apply(request, json_output(links)),
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
                            fetch_links(format!("{} {}", stored, query), version)
                        } else {
                            content_encoding::apply(request, json_output(queries))
                        }
                    },
                    Err(e) => err_response(e)
                }
            } else { fetch_links(query, version) }
        },
        _ => {
           Response::empty_404()
        }
    );
    Ok(resp)
}
