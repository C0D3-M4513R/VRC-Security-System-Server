use crate::rocket::Response;

const BASE_URI: &str = "https://discord.com/oauth2/authorize?";
const SCOPES: &'static str = "identify";
#[actix_web::get("/api/auth/discord/new_oauth")]
pub async fn new_oauth<'r>(discord: &actix_web::web::Data<super::Discord>, auth: Result<super::JWT, super::AuthErr>) -> Response<()> {
    match auth {
        Err(_) => {},
        Ok(_) => return Response::Redirect(None, "/".into()),
    }
    let token = "state";
    let uri = BASE_URI.append("/".into(), Some(format!("client_id={}&response_type=code&redirect_uri={}&scope={SCOPES}&prompt=none&state={token}", discord.id.get(), discord.oauth_redirect_url.to_string()).into()));
    Response::Redirect(None, uri)
}
