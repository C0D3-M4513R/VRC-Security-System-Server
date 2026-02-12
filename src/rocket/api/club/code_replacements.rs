use std::borrow::Cow;
use rocket::Responder;
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::{AuthErr, JWT};
use crate::modals::err::Err;
use crate::rocket::api::club::Permissions;

#[derive(Responder)]
pub enum Response {
    Ok(rocket::response::Redirect),
    AuthErr(AuthErr),
    Error((rocket::http::Status, AskamaWrapper<Err<'static>>)),
    Err((rocket::http::Status, Cow<'static, str>)),
}

#[rocket::put("/api/club/<club>/code_replacements/<target_club>")]
pub async fn put_club_replacement<'r>(auth: Result<JWT, AuthErr>, club: &'r str, target_club: &'r str) -> Response {
    let auth = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };

    match Permissions::require_permission(&auth, club, |v|v.add_allowed_code_replacements).await {
        Ok(()) => {}
        Err(err) => return Response::Error(err),
    }
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT add_allowed_code_replacements($1, $2, $3)",
        auth.get_user_id().cast_signed(), club, target_club
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to add_allowed_code_replacements: {err}");
            return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to add_allowed_code_replacements in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };
    let redir = Response::Ok(rocket::response::Redirect::to(format!("/clubs/{club}")));
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("code_replacements add query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}

#[rocket::delete("/api/club/<club>/code_replacements/<target_club>")]
pub async fn delete_club_replacement<'r>(auth: Result<JWT, AuthErr>, club: &'r str, target_club: &'r str) -> Response {
    let auth = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };

    match Permissions::require_permission(&auth, club, |v|v.remove_allowed_code_replacements).await {
        Ok(()) => {}
        Err(err) => return Response::Error(err),
    }
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT remove_allowed_code_replacements($1, $2, $3)",
        auth.get_user_id().cast_signed(), club, target_club
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to remove_allowed_code_replacements: {err}");
            return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to remove_allowed_code_replacements in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };

    let redir = Response::Ok(rocket::response::Redirect::to(format!("/clubs/{club}")));
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("code_replacements remove query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}