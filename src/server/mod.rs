mod handlers;
mod json;
mod request;
mod response;

use log::info;
use rouille::Response;

pub fn start(vault: super::vault::Vault) {
    info!("Starting a server: http://0.0.0.0:8001");

    rouille::start_server("0.0.0.0:8001", move |request| {
        let res = handlers::api_handler(&request, &vault);
        match res {
            Ok(response) => response,
            Err(err) => Response::text(err.to_string()).with_status_code(500),
        }
    })
}
