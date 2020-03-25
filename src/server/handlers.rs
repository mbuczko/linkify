use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::Vault;

use failure::Error;
use miniserde::json;
use rouille::content_encoding;
use rouille::{post_input, router, try_or_400, Request, Response, ResponseBody};

pub type HandlerResult = Result<Response, Error>;

pub fn handler(request: &Request, vault: &Vault) -> HandlerResult {
    let mut link = Link::default();
    let token = request
        .header("authorization")
        .map_or(None, |header| header.split_whitespace().last());
    let authentication = Authentication::from_token(token);
    let tags = request.get_param("tags");
    let desc = request.get_param("description");
    let omni = request.get_param("omni");
    let limit = request
        .get_param("limit")
        .and_then(|v| v.parse::<u16>().ok());

    let resp = router!(request,
        (GET) (/searches) => {
            let result = vault.list_searches(&authentication);
            //debug!("{:?}", result);
            match result {
                Ok(searches) => {
                    let json = json::to_string(&searches);
                    let resp = Response {
                        status_code: 200,
                        headers: vec![("Content-Type".into(), "application/json".into())],
                        data: ResponseBody::from_string(json),
                        upgrade: None,
                    };
                    content_encoding::apply(request, resp)
                }
                _ => Response::empty_400()
            }
        },
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
        (GET) (/links) => {
            let result = if omni.is_some() {
                vault.omni_search(omni.unwrap(), &authentication, limit)
            } else {
                vault.match_links(link.with_tags(tags).with_description(desc), &authentication, limit, false)
            };
            match result {
                Ok(links) => {
                    let json = json::to_string(&links);
                    let resp = Response {
                        status_code: 200,
                        headers: vec![("Content-Type".into(), "application/json".into())],
                        data: ResponseBody::from_string(json),
                        upgrade: None,
                    };
                    content_encoding::apply(request, resp)
                }
                _ => Response::empty_400()
            }
        },
        _ => {
           Response::empty_404()
        }
    );
    Ok(resp)
}
