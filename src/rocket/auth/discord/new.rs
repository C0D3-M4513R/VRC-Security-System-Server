use rocket::http::uri::fmt::ValidRoutePrefix;

const ROOT: rocket::http::uri::Origin<'static> = rocket::uri!("/");
const BASE_URI: rocket::http::uri::Absolute<'static> = rocket::uri!("https://discord.com/oauth2/authorize?");
const SCOPES: &'static str = "identify";
#[rocket::get("/api/auth/discord/new_oauth")]
pub async fn new_oauth<'r>(discord: &rocket::State<super::Discord>, auth: Result<super::JWT, super::AuthErr>) -> rocket::response::Redirect {
    match auth {
        Err(_) => {},
        Ok(_) => return rocket::response::Redirect::temporary(ROOT),
    }
    let token = "state";
    let uri = BASE_URI.append("/".into(), Some(format!("client_id={}&response_type=code&redirect_uri={}&scope={SCOPES}&prompt=none&state={token}", discord.id.get(), discord.oauth_redirect_url.to_string()).into()));
    rocket::response::Redirect::temporary(uri)
}
