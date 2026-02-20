use std::borrow::Cow;
use crate::rocket::{Response, State};
use crate::rocket::api::club::Permissions;
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::JWT;

#[derive(serde_derive::Deserialize)]
pub struct Name {
    name: String,
}
#[actix_web::post("/api/club/{club}/club_name")]
pub async fn put_club_name<'r>(auth: State<JWT>, path: actix_web::web::Path<String>, data: actix_web::web::Form<Name>) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let club = &*path;
    if club.starts_with("!") != data.name.starts_with("!") {
        return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(crate::modals::err::Err {
            error: Cow::Borrowed("The either the specified Club-Name starts with !, but the current one does or vice-versa. Both options aren't allowed!"),
            error_description: None,
        }))
    }
    if !data.name.is_ascii() {
        return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(crate::modals::err::Err {
            error: Cow::Borrowed("The Specified Club-Name contains non-ascii characters, which isn't allowed!"),
            error_description: None,
        }));
    }
    
    match Permissions::require_permission(&*auth, &club, |v|v.update_club_name).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
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
            return Response::Error(Some(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR), AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to change_club_name in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };

    let redir = Response::Redirect(None, format!("/auth/clubs/{club}").into());
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