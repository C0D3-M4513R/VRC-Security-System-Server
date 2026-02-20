use std::borrow::Cow;
use crate::modals::err::Err;
use crate::rocket::AskamaWrapper;

#[actix_web::get("/api/auth/discord/oauth?<error>&<error_description>&<state>")]
pub async fn oauth_err<'r>(error: String, error_description: Option<String>, state: String) -> AskamaWrapper<Err<'r>> {
    let _ = state;
    AskamaWrapper(Err{
        error: Cow::Owned(error),
        error_description: error_description.map(Cow::Owned),
    })
}