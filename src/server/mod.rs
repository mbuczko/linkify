use rocket::error::LaunchError;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

pub fn start() -> LaunchError {
    rocket::ignite().mount("/", routes![index]).launch()
}
