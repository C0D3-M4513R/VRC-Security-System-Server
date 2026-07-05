use crate::rocket::{Response, State};

const BASE_URI: &str = "https://discord.com/oauth2/authorize?";
const SCOPES: &'static str = "identify";
#[actix_web::get("/api/auth/discord/new_oauth")]
pub async fn new_oauth<'r>(discord: State<super::Discord>) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let token = "state";
    let uri = format!("{BASE_URI}client_id={}&response_type=code&redirect_uri={}&scope={SCOPES}&prompt=none&state={token}", discord.id.get(), discord.oauth_redirect_url.to_string()).into();
    Response::Redirect(None, uri)
}
#[actix_web::get("/api/auth/discord/new_oauth_prompt")]
pub async fn new_oauth_prompt<'r>(discord: State<super::Discord>) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let token = "state";
    let uri = format!("{BASE_URI}client_id={}&response_type=code&redirect_uri={}&scope={SCOPES}&prompt=consent&state={token}", discord.id.get(), discord.oauth_redirect_url.to_string()).into();
    Response::Redirect(None, uri)
}
