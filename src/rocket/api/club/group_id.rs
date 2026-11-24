use std::borrow::Cow;
use crate::rocket::{Response, State};
use crate::rocket::api::club::Permissions;
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::JWT;

#[derive(serde_derive::Deserialize)]
pub struct GroupId {
    group_id: String,
}
#[actix_web::post("/api/club/{club}/group_id")]
pub async fn put_group_id<'r>(auth: State<JWT>, path: actix_web::web::Path<String>, data: actix_web::web::Form<GroupId>) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let club = &*path;
    match Permissions::require_permission(&*auth, &club, |v|v.update_group_id).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }
    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT change_group_id($1, $2, $3)",
        auth.get_user_id().cast_signed(), club, data.group_id
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to change_group_id: {err}");
            return Response::Error(Some(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR), AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to change_group_id in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };

    let redir = Response::Redirect(None, format!("/auth/clubs/{club}/").into());
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("change_group_id query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}