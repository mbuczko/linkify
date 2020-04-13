use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::Vault;
use crate::db::DBLookupType;

use failure::Error;
use miniserde::{json, Serialize};
use rouille::content_encoding;
use rouille::{post_input, router, try_or_400, Request, Response, ResponseBody};

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

pub fn handler(request: &Request, vault: &Vault) -> HandlerResult {
    let mut link = Link::default();
    let token = request
        .header("authorization")
        .map_or(None, |header| header.split_whitespace().last());
    let authentication = Authentication::from_token(token);
    let tags = request.get_param("tags");
    let omni = request.get_param("omni");
    let title = request.get_param("title");
    let notes = request.get_param("notes");
    let limit = request
        .get_param("limit")
        .and_then(|v| v.parse::<u16>().ok());

    let resp = router!(request,
        (POST) (/searches) => {
            match post_input!(request, {name: String, query: String}) {
                Ok(t) => {
                    let id = vault.store_search(&authentication, t.name, t.query)?;
                    Response {
                        status_code: 200,
                        headers: vec![("Location".into(), format!("/searches/{}", id).into())],
                        data: ResponseBody::empty(),
                        upgrade: None,
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
                _ => Response::empty_400()
            }
        },
        (GET) (/links) => {
            let result = if omni.is_some() {
                vault.omni_search(omni.unwrap(), &authentication, limit)
            } else {
                vault.match_links(
                    link
                        .with_title(title)
                        .with_tags(tags)
                        .with_notes(notes),
                    &authentication, limit, false)
            };
            match result {
                Ok(links) => content_encoding::apply(request, jsonize(links)),
                _ => Response::empty_400()
            }
        },
        _ => {
           Response::empty_404()
        }
    );
    Ok(resp)
}
