use std::borrow::Cow;
use base64::Engine;
use rocket::{Request, Responder};
use rocket::request::Outcome;
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

#[derive(Debug, Clone)]
pub struct ETag<'a> {
    pub status: rocket::http::Status,
    pub data: Option<Cow<'a, [u8]>>,
    pub sha3_512: Cow<'a, [u8]>,
    pub header: rocket::http::HeaderMap<'a>
}
#[rocket::async_trait]
impl<'r, 'o:'r> rocket::response::Responder<'r, 'o> for &'o ETag<'o> {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'o> {
        let mut response = rocket::response::Response::new();
        let header_base64 = base64::engine::general_purpose::STANDARD.encode(&self.sha3_512);
        for header in self.header.iter() {
            response.adjoin_header(header);
        }
        response.set_status(self.status);
        response.adjoin_header(rocket::http::Header::new("ETag", format!("sha3_512-{header_base64}")));
        if let Some(data) = self.data.as_ref() {
            response.set_sized_body(data.len(), std::io::Cursor::new(data));
        } else {
            response.set_status(rocket::http::Status::NotModified);
        }
        Ok(response)
    }
}
#[rocket::async_trait]
impl<'r, 'o:'r> rocket::response::Responder<'r, 'o> for ETag<'o> {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'o> {
        let mut response = rocket::response::Response::new();
        let header_base64 = base64::engine::general_purpose::STANDARD.encode(&self.sha3_512);
        for header in self.header.into_iter() {
            response.adjoin_header(header);
        }
        response.set_status(self.status);
        response.adjoin_header(rocket::http::Header::new("ETag", format!("sha3_512-{header_base64}")));
        if let Some(data) = self.data {
            response.set_sized_body(data.len(), std::io::Cursor::new(data));
        } else {
            response.set_status(rocket::http::Status::NotModified);
        }
        Ok(response)
    }
}

pub struct IfNoneMatch(pub Vec<Vec<u8>>);
#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for IfNoneMatch {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut vec = Vec::new();
        for header in request.headers().get("If-None-Match") {
            for header in header.split(","){
                let header = header.trim();
                if header.starts_with("W/"){
                    continue;
                }
                let header = if let Some(v) = header.strip_prefix(r#"""#) {
                    if let Some(v) = v.strip_suffix(r#"""#) {
                        v
                    } else {
                        continue;
                    }
                } else {
                    header
                };
                if let Some(header) = header.strip_prefix("sha3_512-"){
                    if let Ok(v) = base64::engine::general_purpose::STANDARD.decode(header) {
                        vec.push(v);
                    }
                }

            }
        }
        Outcome::Success(IfNoneMatch(vec))
    }
}

#[derive(Responder)]
pub enum Response<T> {
    Ok(T),
    AuthErr(AuthErr),
    Redirect(rocket::response::Redirect),
    Error((rocket::http::Status, AskamaWrapper<crate::modals::err::Err<'static>>)),
}