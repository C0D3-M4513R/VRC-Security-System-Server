use std::borrow::Cow;
use crate::rocket::{AskamaWrapper, State};
use crate::modals::err::Err;
use crate::rocket::Response;

#[actix_web::get("/api/auth/discord/oauth?{code}&{state}")]
pub async fn oauth_ok<'a>(discord: State<'a, super::Discord>, key: State<'a, actix_web::cookie::Key>, req: &actix_web::HttpRequest, code: &str, state: &str) -> Response<actix_web::HttpResponse<&'static str>> {
    let _ = state; //Yes rust, I don't use this field. I know. Now stop complaining.
    let jwt = match super::JWT::new(discord, code).await {
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