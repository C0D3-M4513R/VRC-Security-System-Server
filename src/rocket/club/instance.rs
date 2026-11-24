use std::borrow::Cow;
use crate::Limits;
use crate::modals::err::Err;
use crate::modals::club_instance::ClubInstance;
use crate::rocket::api::club::build_permission_from_res;
use crate::rocket::{AskamaWrapper, Response, State};
use crate::rocket::auth::discord::JWT;

#[actix_web::get("/clubs/{club}/")]
pub async fn get_club_instance<'r>(auth: State<JWT>, limits: State<Limits>, path: actix_web::web::Path<String>) -> Response<AskamaWrapper<ClubInstance>> {
    let db = crate::get_db().await;
    let club = &*path;
    let res = match sqlx::query!(r#"
        SELECT
            public.club."path-name" as path_name,
            public.club.name as name,
            public.club.code as code,
            public.discord_permissions.*,
            permission_club."path-name" as "club_path"
        FROM club
            INNER JOIN public.discord_permissions on public.club.id = public.discord_permissions.club_id OR public.discord_permissions.club_id = 0
            INNER JOIN public.club AS permission_club ON permission_club.id = public.discord_permissions.club_id
            WHERE public.discord_permissions.discord_id = $1 AND public.club."path-name" = $2
            ORDER BY public.discord_permissions.club_id
            LIMIT 1
    "#,
        auth.get_user_id().cast_signed(),
        club,
    ).fetch_optional(&db)
        .await {
        Ok(Some(res)) => res,
        Ok(None) => return Response::Error(Some(actix_web::http::StatusCode::FORBIDDEN), AskamaWrapper(Err{
            error: Cow::Borrowed("You do not have permission to access this Club!"),
            error_description: None,
        })),
        Err(_) => return Response::Error(None, AskamaWrapper(Err {
            error: Cow::Borrowed("Failed to fetch your permissions across club's from the Database"),
            error_description: None
        })),
    };

    let perms = build_permission_from_res!(res);

    let custom_json = if perms.club_path == crate::rocket::api::club::CLUB_OWNERS && perms.manage_permissions.is_some() {
        sqlx::query_scalar!(
            r#"SELECT jsonb_pretty(custom_json) FROM club WHERE "path-name" = $1"#,
            res.path_name
        )
        .fetch_optional(&db)
        .await
        .ok()
        .flatten()
        .flatten()
    } else {
        None
    };

    Response::Ok(AskamaWrapper(ClubInstance{
        info: crate::modals::club_instance::ClubInfo{
            name: res.name,
            path_name: res.path_name,
            permissions: perms,
            limits: limits.clone(),
        },
        code: res.code,
        custom_json,
    }))
}