use std::borrow::Cow;
use crate::rocket::{AskamaWrapper, State};
use crate::rocket::auth::discord::JWT;
use crate::modals::err::Err;
use crate::rocket::api::club::Permissions;
use crate::rocket::Response;

#[actix_web::put("/api/club/<club>/code_replacements/<target_club>")]
pub async fn put_club_replacement<'r>(auth: State<JWT>, club: String, target_club: String) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    match Permissions::require_permission(&auth, &club, |v|v.add_allowed_code_replacements).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
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
            return Response::Error(None, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to add_allowed_code_replacements in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };
    let redir = Response::Redirect(None, format!("/clubs/{club}").into());
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

#[actix_web::delete("/api/club/<club>/code_replacements/<target_club>")]
pub async fn delete_club_replacement<'r>(auth: State<JWT>, club: String, target_club: String) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    match Permissions::require_permission(&auth, &club, |v|v.remove_allowed_code_replacements).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
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
            return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to remove_allowed_code_replacements in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };

    let redir = Response::Redirect(None, format!("/clubs/{club}").into());
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