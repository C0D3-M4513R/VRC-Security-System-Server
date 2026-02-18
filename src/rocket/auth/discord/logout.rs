use std::borrow::Cow;
use crate::modals::err::Err;
use crate::rocket::AskamaWrapper;

#[actix_web::get("/api/auth/discord/logout")]
pub async fn logout() -> actix_web::HttpResponse<actix_web::body::EitherBody<(), String>> {
    let mut cookie = actix_web::cookie::Cookie::new(super::DISCORD_TOKEN_COOKIE_NAME, "");
    cookie.set_secure(true);
    cookie.make_removal();
    let mut resp = actix_web::HttpResponse::with_body(actix_web::http::StatusCode::TEMPORARY_REDIRECT, ().into());
    match resp.add_removal_cookie(&cookie) {
        Ok(()) => {},
        Err(e) => {
            let err = AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to remove Auth cookie"),
                error_description: Some(e.to_string().into()),
            }).render()
            .map_or_else(core::convert::identity, core::convert::identity);
            *resp.status_mut() = actix_web::http::StatusCode::INTERNAL_SERVER_ERROR;
            return resp.set_body(err.into());
        }
    }
    resp.headers_mut().insert(actix_web::http::header::LOCATION, actix_web::http::header::HeaderValue::from_static("/"));
    resp
}