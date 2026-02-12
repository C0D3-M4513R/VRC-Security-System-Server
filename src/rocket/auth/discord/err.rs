use std::borrow::Cow;
use crate::modals::err::Err;
use crate::rocket::AskamaWrapper;

#[rocket::get("/api/auth/discord/oauth?<error>&<error_description>&<state>", rank=1)]
pub async fn oauth_err<'r>(error: &'r str, error_description: Option<&'r str>, state: &'_ str) -> AskamaWrapper<Err<'r>> {
    let _ = state;
    AskamaWrapper(Err{
        error: Cow::Borrowed(error),
        error_description: error_description.map(Cow::Borrowed),
    })
}