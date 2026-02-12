use rocket::{Request, Responder};
use crate::rocket::auth::discord::AuthErr;

pub mod api;
pub mod auth;
pub mod club;

#[rocket::get("/")]
pub fn get_index() -> rocket::response::Redirect {
    rocket::response::Redirect::to("/clubs")
}
#[rocket::get("/favicon.ico")]
pub fn get_favicon() -> (rocket::http::ContentType, &'static[u8]) {
    (rocket::http::ContentType::Icon, include_bytes!("../favicon.ico"))
}

pub struct AskamaWrapper<T>(pub T);
impl<'r, 'o:'r, T: askama::Template + 'r> ::rocket::response::Responder<'r, 'o> for AskamaWrapper<T> {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        match self.0.render() {
            Ok(v) => (rocket::http::ContentType::HTML,v).respond_to(request),
            Err(err) => (rocket::http::ContentType::HTML,format!(r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <meta name="color-scheme" content="light dark">
        <title>OAuth Error</title>
    </head>
    <body>
        <h1>Error Getting OAuth Token</h1>
        <p>Error: <code>{err}</code></p>
    </body>
"#)).respond_to(request),
        }
    }
}

#[derive(Responder)]
pub enum Response<T> {
    Ok(T),
    AuthErr(AuthErr),
    Error((rocket::http::Status, AskamaWrapper<crate::modals::err::Err<'static>>)),
}