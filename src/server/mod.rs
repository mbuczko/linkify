pub mod handlers;

use crate::server::handlers::handler;
use crate::vault::Vault;

use log::info;
use rouille::Response;

pub fn start(vault: Vault) {
    info!("Starting a server: http://localhost:8001");

    rouille::start_server("127.0.0.1:8001", move |request| {
        let res = handler(&request, &vault);
        match res {
            Ok(response) => response,
            Err(err) => Response::text(err.to_string()).with_status_code(500),
        }
    })
}
