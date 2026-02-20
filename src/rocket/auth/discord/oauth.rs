use std::borrow::Cow;
use crate::rocket::{AskamaWrapper, State};
use crate::modals::err::Err;
use crate::rocket::Response;

#[derive(serde_derive::Deserialize)]
#[serde(untagged)]
pub enum Query{
    Ok(QueryOk),
    Err(QueryErr),
}

#[derive(serde_derive::Deserialize)]
pub struct QueryOk{
    code: String,
    state: String,
}

#[derive(serde_derive::Deserialize)]
pub struct QueryErr{
    error: String,
    error_description: Option<String>,
    state: String,
}


#[actix_web::get("/api/auth/discord/oauth")]
pub async fn oauth(discord: State<super::Discord>, key: State<actix_web::cookie::Key>, query: actix_web::web::Query<Query>) -> Response<actix_web::HttpResponse<&'static str>> {
    let query = match query.into_inner() {
        Query::Ok(v) => v,
        Query::Err(err) => return Response::Error(Some(actix_web::http::StatusCode::UNAUTHORIZED), AskamaWrapper(Err{
            error: Cow::Owned(err.error),
            error_description: err.error_description.map(Cow::Owned),
        })),
    };
    let jwt = match super::JWTInternal::new(&*discord, &query.code).await {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Could not get Discord Auth: {err}");
            return Response::Error(None, AskamaWrapper(Err{
                error: Cow::Borrowed("Could not get Discord Auth"),
                error_description: None,
            }));
        }
    };

    let jwt_string = match serde_json::to_string(&jwt) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Failed serializing jwt: {err}");
            return Response::Error(None, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed serializing jwt"),
                error_description: None,
            }));
        }
    };
    let mut jar = actix_web::cookie::CookieJar::new();
    {
        let mut private_jar = jar.private_mut(&*key);
        let mut cookie = actix_web::cookie::Cookie::new(super::DISCORD_TOKEN_COOKIE_NAME, jwt_string);
        cookie.set_secure(true);
        cookie.set_http_only(true);
        cookie.set_same_site(actix_web::cookie::SameSite::Strict);
        cookie.set_path("/");
        cookie.set_expires(actix_web::cookie::Expiration::DateTime(actix_web::cookie::time::OffsetDateTime::now_utc().saturating_add(actix_web::cookie::time::Duration::days(7))));
        private_jar.add(cookie);
    }
    let mut response = actix_web::HttpResponse::with_body(actix_web::http::StatusCode::OK, include_str!("../../../../templates/api/auth/discord/auth-success.html"));
    response.headers_mut().insert(actix_web::http::header::CONTENT_TYPE, actix_web::http::header::HeaderValue::from_static("text/html"));
    for i in jar.delta() {
        match response.add_cookie(i) {
            Ok(()) => {},
            Err(err) => {
                return Response::Error(None, AskamaWrapper(Err{
                    error: Cow::Borrowed("Failed to add set-cookie header"),
                    error_description: Some(err.to_string().into()),
                }));
            }
        }
    }

    Response::Ok(response)
}