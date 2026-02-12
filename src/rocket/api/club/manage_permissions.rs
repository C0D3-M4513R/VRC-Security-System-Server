use std::borrow::Cow;
use crate::rocket::api::club::code_replacements::Response;
use crate::rocket::api::club::Permissions;
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::{AuthErr, JWT};
use crate::modals::err::Err;

#[rocket::put("/api/club/<club>/manage_permissions/<target_id>", data="<data>")]
pub async fn put_club_permission<'r>(auth: Result<JWT, AuthErr>, club: &'r str, target_id: u64, data: rocket::form::Form<Permissions>) -> Response {
    process_club_permission(auth, club, target_id, Some(data.into_inner())).await
}
async fn process_club_permission(auth: Result<JWT, AuthErr>, club: &str, target_id: u64, data: Option<Permissions>) -> Response {
    let auth = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };
    let perms = match Permissions::get_from_db(target_id, club).await {
        Ok(perms) => perms,
        Err(_) => return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to get Permissions of target discord id"),
            error_description: None,
        }))),
    };
    match Permissions::require_permission(&auth, club, |v| match perms.map(|v|v.manage_permissions).flatten() {
        None => v.manage_permissions.is_some(),
        Some(level) => match v.manage_permissions {
            None => false,
            Some(self_level) => self_level < level,
        },
    }).await {
        Ok(()) => {}
        Err(err) => return Response::Error(err),
    }

    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT manage_permissions($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
        auth.get_user_id().cast_signed(),
        target_id.cast_signed(),
        club,
        data.as_ref().map(|v|v.add_discord_user).unwrap_or(false),
        data.as_ref().map(|v|v.remove_discord_user).unwrap_or(false),
        data.as_ref().map(|v|v.update_club_name).unwrap_or(false),
        data.as_ref().map(|v|v.add_allowed_code_replacements).unwrap_or(false),
        data.as_ref().map(|v|v.add_level).flatten(),
        data.as_ref().map(|v|v.update_logo).unwrap_or(false),
        data.as_ref().map(|v|v.update_poster1).unwrap_or(false),
        data.as_ref().map(|v|v.update_poster2).unwrap_or(false),
        data.as_ref().map(|v|v.update_poster3).unwrap_or(false),
        data.as_ref().map(|v|v.remove_allowed_code_replacements).unwrap_or(false),
        data.as_ref().map(|v|v.remove_level).flatten(),
        data.as_ref().map(|v|v.manage_permissions).flatten(),
        data.as_ref().map(|v|v.submit).unwrap_or(false),
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to manage_permissions: {err}");
            return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to manage_permissions in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };

    let redir = Response::Ok(rocket::response::Redirect::to(format!("/clubs/{club}/discord_permissions")));
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("manage_permissions query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}
#[derive(rocket::FromForm, Debug)]
pub struct NewPermission {
    target_id: u64,
}
#[rocket::put("/api/club/<club>/manage_permissions", data="<data>")]
pub async fn new_club_permission<'r>(auth: Result<JWT, AuthErr>, club: &'r str, data: rocket::form::Form<NewPermission>) -> Response {
    process_club_permission(auth, club, data.target_id, None).await
}
#[rocket::delete("/api/club/<club>/manage_permissions/<target_id>")]
pub async fn delete_club_permission<'r>(auth: Result<JWT, AuthErr>, club: &'r str, target_id: u64) -> Response {
    let auth = match auth {
        Ok(jwt) => jwt,
        Err(err) => return Response::AuthErr(err),
    };
    let perms = match Permissions::get_from_db(target_id, club).await {
        Ok(perms) => perms,
        Err(_) => return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to get Permissions of target discord id"),
            error_description: None,
        }))),
    };
    match Permissions::require_permission(&auth, club, |v| match perms.map(|v|v.manage_permissions).flatten() {
        None => v.manage_permissions.is_some(),
        Some(level) => match v.manage_permissions {
            None => false,
            Some(self_level) => self_level < level,
        },
    }).await {
        Ok(()) => {}
        Err(err) => return Response::Error(err),
    }
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT delete_permissions($1, $2, $3)",
        auth.get_user_id().cast_signed(),
        target_id.cast_signed(),
        club
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to delete_permissions: {err}");
            return Response::Error((rocket::http::Status::BadRequest, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to delete_permissions in DB"),
                error_description: Some(err.to_string().into()),
            })))
        }
    };

    let redir = Response::Ok(rocket::response::Redirect::to(format!("/clubs/{club}/discord_permissions")));
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("delete_permissions query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}