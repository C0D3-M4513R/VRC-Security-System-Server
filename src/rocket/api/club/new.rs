use std::borrow::Cow;
use crate::rocket::{Response, State};
use crate::rocket::api::club::{Permissions, CLUB_OWNERS};
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::JWT;

#[derive(serde_derive::Deserialize)]
pub struct Name {
    path_name: String,
}
#[actix_web::post("/api/club")]
pub async fn put_club<'r>(auth: State<JWT>, data: actix_web::web::Form<Name>) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    if data.path_name.starts_with("!") {
        return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(crate::modals::err::Err {
            error: Cow::Borrowed("The either the specified Club-Name starts with !. That isn't allowed, except for some special cases."),
            error_description: None,
        }))
    }
    if !data.path_name.is_ascii() {
        return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(crate::modals::err::Err {
            error: Cow::Borrowed("The Specified Club-Name contains non-ascii characters, which isn't allowed!"),
            error_description: None,
        }));
    }

    match Permissions::require_permission(&auth, CLUB_OWNERS, |v|v.manage_permissions == Some(0)).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }

    let (pk, sk) = match sphincsplus::crypto_sign_keypair() {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to generate a sphincsplus key-pair: {err}");
            return Response::Error(None, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to generate a sphincsplus key-pair"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };
    let club = &data.path_name;
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT club_create($1, club_get_new_code(), $2, $3, $4)",
        auth.get_user_id().cast_signed(), club, &pk, &sk
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to club_create: {err}");
            return Response::Error(None, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to club_create in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };
    let redir = Response::Redirect(None, format!("/auth/clubs/{club}/").into());
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("club_create add query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}

