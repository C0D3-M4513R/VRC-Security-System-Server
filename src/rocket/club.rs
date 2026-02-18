pub mod instance;
pub mod vrchat_permissions;
pub mod manage_permissions;

use std::borrow::Cow;
use crate::rocket::{AskamaWrapper, Response};
use crate::rocket::auth::discord::{AuthErr, JWT};
use crate::rocket::api::club::build_permission_from_res;
use crate::modals::err::Err;
use crate::modals::clubs::{Club, Clubs};

#[actix_web::get("/clubs")]
pub async fn get_club(auth: State<JWT>) -> Response<AskamaWrapper<Clubs>> {
    let auth = match auth {
        Ok(a) => a,
        Err(e) => return Response::AuthErr(e),
    };

    let db = crate::get_db().await;
    let res = match sqlx::query!(r#"
        SELECT
            public.club."path-name" as path_name,
            public.club.code as code,
            public.club.name as name
        FROM club
            INNER JOIN public.discord_permissions on public.club.id = public.discord_permissions.club_id OR public.discord_permissions.club_id = 0
            WHERE public.discord_permissions.discord_id = $1
            ORDER BY name
    "#, auth.get_user_id().cast_signed()).fetch_all(&db)
        .await {
        Ok(res) => res,
        Err(_) => return Response::Error((actix_web::http::StatusCode::InternalServerError, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to fetch your permissions across club's from the Database"),
            error_description: None
        }))),
    };
    let permission = match sqlx::query!(r#"
        SELECT
            public.discord_permissions.*
        FROM public.discord_permissions
        WHERE public.discord_permissions.discord_id = $1 AND public.discord_permissions.club_id = 0
    "#, auth.get_user_id().cast_signed()).fetch_optional(&db)
        .await {
        Ok(Some(v)) => Some(build_permission_from_res!(v)),
        Ok(None) => None,
        Err(_) => return Response::Error((actix_web::http::StatusCode::InternalServerError, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to fetch your permissions across from the Database"),
            error_description: None
        }))),
    };

    Response::Ok(AskamaWrapper(Clubs{
        clubs: res.into_iter().map(|c| Club{path_name: c.path_name, code: c.code.cast_unsigned(), name: c.name}).collect(),
        permission
    }))
}