use std::borrow::Cow;
use crate::rocket::AskamaWrapper;
use crate::modals::err::Err;
use crate::rocket::Response;

#[rocket::get("/api/auth/discord/oauth?<code>&<state>", rank=0)]
pub async fn oauth_ok(discord: &rocket::State<super::Discord>, cookie_jar: &rocket::http::CookieJar<'_>, code: &str, state: &str) -> Response<rocket::response::content::RawHtml<&'static str>> {
    let _ = state; //Yes rust, I don't use this field. I know. Now stop complaining.
    let jwt = match super::JWT::new(discord, code).await {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Could not get Discord Auth: {err}");
            return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                error: Cow::Borrowed("Could not get Discord Auth"),
                error_description: None,
            })));
        }
    };

    let jwt_string = match rocket::serde::json::to_string(&jwt) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("Failed serializing jwt: {err}");
            return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed serializing jwt"),
                error_description: None,
            })));
        }
    };
    let mut cookie = rocket::http::Cookie::new(super::DISCORD_TOKEN_COOKIE_NAME, jwt_string);
    cookie.set_secure(true);
    cookie_jar.add_private(cookie);
    Response::Ok(rocket::response::content::RawHtml(r##"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="refresh" content="0; url=/" />
    <meta name="color-scheme" content="light dark">
    <title>Discord Auth Success</title>
</head>
<body>
    <h1>Successfully Authenticated via Discord</h1>
    <p>If you see this, then you are likely using an old Browser. </p><a href="/">Please click here to be Redirected.</a></p>
</body>
</html>
    "##))
}