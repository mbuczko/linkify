use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::Vault;

use failure::Error;
use miniserde::json;
use rouille::content_encoding;
use rouille::{router, Request, Response, ResponseBody};

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
        (GET) (/) => {
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
                        headers: vec![("Content-Type".into(), "application/json; charset=utf-8".into())],
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
