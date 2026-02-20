use std::borrow::Cow;
use crate::Limits;
use crate::rocket::{AskamaWrapper, Response, State};
use crate::rocket::auth::discord::JWT;
use crate::modals::err::Err;
use crate::rocket::api::club::Permissions;

#[derive(serde_derive::Deserialize)]
pub struct VRCUserLevel {
    vrc_name: String,
    permission_level: i16,
}

#[actix_web::put("/api/club/<club>/vrcuser_level")]
pub async fn put_vrcuser_level<'r>(limits: State<Limits>, auth: State<JWT>, club: String, data: actix_web::web::Form<VRCUserLevel>) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let data = data.into_inner();
    if data.permission_level < 0 {
        return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err {
            error: Cow::Borrowed("The specified permission level is too low"),
            error_description: Some(Cow::Owned(format!("The level {} cannot be lower than 0", data.permission_level))),
        }));
    }
    if data.permission_level as u64 > limits.max_permission_level {
        return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err {
            error: Cow::Borrowed("The specified permission level is too high"),
            error_description: Some(Cow::Owned(format!("The level {} exceeds the allowed maximum of {}", data.permission_level, limits.max_permission_level))),
        }));
    }
    match Permissions::require_permission(&auth, &club, |v|match v.add_level{
        None => false,
        Some(v) => v <= data.permission_level
    }).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT add_vrcuser_level($1, $2, $3, $4)",
        auth.get_user_id().cast_signed(),
        club,
        data.vrc_name,
        i32::from(data.permission_level),
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to add_vrcuser_level: {err}");
            return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to add_vrcuser_level Image in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };

    let redir = Response::Redirect(None, format!("/clubs/{club}/vrchat_permissions").into());
    match table.rows_affected() {
        0 => {},
        _ => return redir,
    }


    redir
}

#[actix_web::delete("/api/club/<club>/vrcuser_level/<level>/<vrc_username>")]
pub async fn delete_vrcuser_level<'r>(limits: State<Limits>, auth: State<JWT>, club: String, level: String, vrc_username: String) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let level = match u32::from_str_radix(&level, 10) {
        Ok(v) => v,
        Err(err) => return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to decode the parsed level as an unsigned integer"),
            error_description: Some(err.to_string().into()),
        }))
    };
    if level as u64 > limits.max_permission_level {
        return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err {
            error: Cow::Borrowed("The specified permission level is too high"),
            error_description: Some(Cow::Owned(format!("The level {level} exceeds the allowed maximum of {}", limits.max_permission_level))),
        }));
    }
    match Permissions::require_permission(&auth, &club, |v|match v.remove_level{
        None => false,
        Some(v) => u32::from(v.cast_unsigned()) <= level
    }).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT remove_vrcuser_level($1, $2, $3, $4)",
        auth.get_user_id().cast_signed(), club, vrc_username, level.cast_signed()
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to remove_vrcuser_level: {err}");
            return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to remove_vrcuser_level in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };

    let redir = Response::Redirect(None, format!("/clubs/{club}/vrchat_permissions").into());
    match table.rows_affected() {
        0 => {},
        _ => return redir,
    }
    
    redir
}
