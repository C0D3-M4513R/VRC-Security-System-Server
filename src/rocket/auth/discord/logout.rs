#[rocket::get("/api/auth/discord/logout")]
pub async fn logout(cookies: &rocket::http::CookieJar<'_>) -> rocket::response::Redirect {
    if let Some(v) = cookies.get_private(super::DISCORD_TOKEN_COOKIE_NAME){
        cookies.remove_private(v);
    }

    rocket::response::Redirect::to("/")
}