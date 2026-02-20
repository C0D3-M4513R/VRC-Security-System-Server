use std::borrow::Cow;
use crate::Limits;
use crate::modals::club_instance::ClubInstance;
use crate::modals::err::Err;
use crate::modals::club_vrchat_permissions::{*};
use crate::rocket::api::club::build_permission_from_res;
use crate::rocket::{AskamaWrapper, Response, State};
use crate::rocket::auth::discord::JWT;

#[actix_web::get("/clubs/<club>/vrchat_permissions")]
pub async fn get_club_vrc_names<'r>(auth: State<JWT>, limits: State<Limits>, club: String) -> Response<AskamaWrapper<ClubVRCPermissions>> {
    let db = crate::get_db().await;
    let res = match sqlx::query!(r#"
        SELECT
            public.club."path-name" as path_name,
            public.club.name as name,
            public.club.id as actual_club_id,
            public.club.code as code,
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
        Ok(None) => return Response::Error(Some(actix_web::http::StatusCode::FORBIDDEN), AskamaWrapper(Err{
            error: Cow::Borrowed("You do not have permission to access this Club!"),
            error_description: None,
        })),
        Err(_) => return Response::Error(None, AskamaWrapper(Err {
            error: Cow::Borrowed("Failed to fetch your permissions across club's from the Database"),
            error_description: None
        })),
    };


    let vrc_perms = match sqlx::query!(r#"
        SELECT
            public.club_vrc_permission.permission_level,
            public.vrc_name.name as vrc_name
        FROM club
            INNER JOIN public.club_vrc_permission ON public.club.id = public.club_vrc_permission.club_id
            INNER JOIN public.vrc_name ON public.club_vrc_permission.vrc_name = public.vrc_name.id
            WHERE public.club.id = $1
            ORDER BY public.club_vrc_permission.permission_level ASC
    "#,
        res.actual_club_id,
    ).fetch_all(&db)
        .await {
        Ok(res) => res,
        Err(_) => return Response::Error(None, AskamaWrapper(Err {
            error: Cow::Borrowed("Failed to fetch your permissions across club's from the Database"),
            error_description: None
        })),
    };

    let perms = build_permission_from_res!(res);

    Response::Ok(AskamaWrapper(ClubVRCPermissions{
        information: ClubInstance {
            name: res.name,
            code: res.code,
            path_name: res.path_name,
            permissions: perms,
            limits: limits.clone(),
        },
        permissions: vrc_perms.into_iter().map(|v|VRCPermission{ vrc_name: v.vrc_name, permission_level: v.permission_level }).collect(),
    }))
}