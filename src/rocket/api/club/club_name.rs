use std::borrow::Cow;
use crate::rocket::api::club::code_replacements::Response;
use crate::rocket::api::club::Permissions;
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::{AuthErr, JWT};

#[derive(rocket::FromForm)]
pub struct Name<'r> {
    name: &'r str,
}
#[rocket::put("/api/club/<club>/club_name", data = "<data>")]
pub async fn put_club_name<'r>(auth: Result<JWT, AuthErr>, club: &'r str, data: rocket::form::Form<Name<'r>>) -> Response {
    let auth = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };
    if data.name.starts_with("!") {
        return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(crate::modals::err::Err {
            error: Cow::Borrowed("The Specified Club-Name starts with !, which isn't allowed!"),
            error_description: None,
        })))
    }
    if !data.name.is_ascii() {
        return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(crate::modals::err::Err {
            error: Cow::Borrowed("The Specified Club-Name contains non-ascii characters, which isn't allowed!"),
            error_description: None,
        })))
    }
    
    match Permissions::require_permission(&auth, club, |v|v.update_club_name).await {
        Ok(()) => {}
        Err(err) => return Response::Error(err),
    }
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT change_club_name($1, $2, $3)",
        auth.get_user_id().cast_signed(), club, data.name
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to change_club_name: {err}");
            return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to change_club_name in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };

    let redir = Response::Ok(rocket::response::Redirect::to(format!("/clubs/{club}")));
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("change_club_name query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}