use std::borrow::Cow;
use crate::rocket::api::club::Permissions;
use crate::rocket::auth::discord::JWT;
use crate::rocket::{AskamaWrapper, Response, State};

#[derive(serde_derive::Deserialize)]
pub struct DiscordInfo {
    discord_id: u64,
    username: String,
    discriminator: Option<i16>,
}
#[actix_web::put("/api/discord/info")]
pub async fn put_discord_info<'r>(auth: State<JWT>, data: actix_web::web::Form<DiscordInfo>) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    match Permissions::require_permission(&auth, crate::rocket::api::club::CLUB_OWNERS, |v|v.manage_permissions == Some(0)).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }

    let db = crate::get_db().await;
    let table = match sqlx::query!(
        "SELECT discord_create($1, $2, $3, $4, null)",
        auth.get_user_id().cast_signed(), data.discord_id.cast_signed(), data.username, data.discriminator
    )
        .execute(&db)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to discord_create: {err}");
            return Response::Error(None, AskamaWrapper(crate::modals::err::Err {
                error: Cow::Borrowed("Failed to discord_create in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };
    let redir = Response::Redirect(None, "/clubs/".into());
    match table.rows_affected() {
        0 => {},
        1 => return redir,
        affected => {
            tracing::error!("discord_create add query affected more than 1 row? {affected}");
            return redir
        }
    }

    redir
}