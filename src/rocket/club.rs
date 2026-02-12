pub mod instance;
pub mod vrchat_permissions;
pub mod manage_permissions;

use std::borrow::Cow;
use rocket::Responder;
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::{AuthErr, JWT};
use crate::modals::err::Err;
use crate::modals::clubs::{Club, Clubs};

#[derive(Responder)]
pub enum Response {
    Ok(AskamaWrapper<Clubs>),
    AuthErr(AuthErr),
    Error((rocket::http::Status, AskamaWrapper<Err<'static>>)),
}

#[rocket::get("/clubs")]
pub async fn get_club(auth: Result<JWT, AuthErr>) -> Response {
    let auth = match auth {
        Ok(a) => a,
        Err(e) => return Response::AuthErr(e),
    };

    let db = crate::get_db().await;
    let res = match sqlx::query!(r#"
        SELECT
            public.club."path-name" as path_name,
            public.club.name as name
        FROM club
            INNER JOIN public.discord_permissions on public.club.id = public.discord_permissions.club_id OR public.discord_permissions.club_id = 0
            WHERE public.discord_permissions.discord_id = $1
            ORDER BY name
    "#, auth.get_user_id().cast_signed()).fetch_all(&db)
        .await {
        Ok(res) => res,
        Err(_) => return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to fetch your permissions across club's from the Database"),
            error_description: None
        }))),
    };

    Response::Ok(AskamaWrapper(Clubs{
        clubs: res.into_iter().map(|c| Club{path_name: c.path_name, name: c.name}).collect(),
    }))
}