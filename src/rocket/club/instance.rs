use std::borrow::Cow;
use crate::Limits;
use crate::modals::err::Err;
use crate::modals::club_instance::ClubInstance;
use crate::rocket::api::club::build_permission_from_res;
use crate::rocket::{AskamaWrapper, Response};
use crate::rocket::auth::discord::{AuthErr, JWT};

#[rocket::get("/clubs/<club>")]
pub async fn get_club_instance<'r>(auth: Result<JWT, AuthErr>, limits: &'r rocket::State<Limits>, club: &str) -> Response<AskamaWrapper<ClubInstance<'r>>> {
    let auth = match auth {
        Ok(a) => a,
        Err(e) => return Response::AuthErr(e),
    };

    let db = crate::get_db().await;
    let res = match sqlx::query!(r#"
        SELECT
            public.club."path-name" as path_name,
            public.club.name as name,
            public.discord_permissions.*
        FROM club
            INNER JOIN public.discord_permissions on public.club.id = public.discord_permissions.club_id OR public.discord_permissions.club_id = 0
            WHERE public.discord_permissions.discord_id = $1 AND public.club."path-name" = $2
            ORDER BY public.discord_permissions.club_id
            LIMIT 1
    "#,
        auth.get_user_id().cast_signed(),
        club,
    ).fetch_optional(&db)
        .await {
        Ok(Some(res)) => res,
        Ok(None) => return Response::Error((rocket::http::Status::Forbidden, AskamaWrapper(Err{
            error: Cow::Borrowed("You do not have permission to access this Club!"),
            error_description: None,
        }))),
        Err(_) => return Response::Error((rocket::http::Status::InternalServerError, AskamaWrapper(Err {
            error: Cow::Borrowed("Failed to fetch your permissions across club's from the Database"),
            error_description: None
        }))),
    };

    let perms = build_permission_from_res!(res);

    Response::Ok(AskamaWrapper(ClubInstance{
        name: res.name,
        path_name: res.path_name,
        permissions: perms,
        limits: &**limits,
    }))
}