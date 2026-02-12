use std::borrow::Cow;
use crate::rocket::Response;
use crate::rocket::api::club::{Permissions, CLUB_OWNERS};
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::{AuthErr, JWT};

#[derive(rocket::FromForm)]
pub struct Name<'r> {
    path_name: &'r str,
}
#[rocket::put("/api/club", data = "<data>")]
pub async fn put_club<'r>(auth: Result<JWT, AuthErr>, data: rocket::form::Form<Name<'r>>) -> Response<rocket::response::Redirect> {
    let auth = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };

    match Permissions::require_permission(&auth, CLUB_OWNERS, |v|v.manage_permissions == Some(0)).await {
        Ok(()) => {}
        Err(err) => return Response::Error(err),
    }
    
    let (pk, sk) = match sphincsplus::crypto_sign_keypair() {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to generate a sphincsplus key-pair: {err}");
            return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to generate a sphincsplus key-pair"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };
    let club = data.path_name;
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
            return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to club_create in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };
    let redir = Response::Ok(rocket::response::Redirect::to(format!("/clubs/{club}")));
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

