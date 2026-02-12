use std::borrow::Cow;
use crate::rocket::AskamaWrapper;
use crate::modals::err::Err;

#[derive(rocket::response::Responder)]
pub enum Responder {
    Ok(rocket::response::Redirect),
    AskamaErr(AskamaWrapper<Err<'static>>),
}

#[rocket::get("/api/discord/oauth?<code>&<state>", rank=0)]
pub async fn oauth_ok(discord: &rocket::State<super::Discord>, cookie_jar: &rocket::http::CookieJar<'_>, code: &str, state: &str) -> Responder {
    let _ = state; //Yes rust, I don't use this field. I know. Now stop complaining.
    let jwt = match super::JWT::new(discord, code).await {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Could not get Discord Auth: {err}");
            return Responder::AskamaErr(AskamaWrapper(Err{
                error: Cow::Borrowed("Could not get Discord Auth"),
                error_description: None,
            }));
        }
    };

    let jwt_string = match rocket::serde::json::to_string(&jwt) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Failed serializing jwt: {err}");
            return Responder::AskamaErr(AskamaWrapper(Err{
                error: Cow::Borrowed("Failed serializing jwt"),
                error_description: None,
            }));
        }
    };
    let mut cookie = rocket::http::Cookie::new(super::DISCORD_TOKEN_COOKIE_NAME, jwt_string);
    cookie.set_secure(true);
    cookie_jar.add_private(cookie);
    Responder::Ok(rocket::response::Redirect::to("/"))
}