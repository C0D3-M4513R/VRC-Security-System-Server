use std::borrow::Cow;
use crate::Limits;
use crate::modals::club_instance::ClubInstance;
use crate::modals::err::Err;
use crate::modals::club_manage_discord_permissions::{*};
use crate::rocket::api::club::build_permission_from_res;
use crate::rocket::{AskamaWrapper, Response, State};
use crate::rocket::auth::discord::JWT;

#[actix_web::get("/clubs/{club}/discord_permissions")]
pub async fn get_club_discord_permissions<'r>(auth: State<JWT>, limits: State<Limits>, path: actix_web::web::Path<String>) -> Response<AskamaWrapper<ClubDiscordPermissions>> {
    let db = crate::get_db().await;
    let club = &*path;
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


    let discord_perms = match sqlx::query!(r#"
        SELECT
            public.discord_permissions.*,
            public.discord_info.username,
            public.discord_info.discriminator
        FROM public.discord_permissions
            INNER JOIN public.discord_info ON public.discord_info.user_id = public.discord_permissions.discord_id
            WHERE public.discord_permissions.club_id = $1
            ORDER BY public.discord_permissions.manage_permissions ASC
    "#,
        res.actual_club_id,
    ).fetch_all(&db)
        .await {
        Ok(res) => res,
        Err(_) => return Response::Error(None, AskamaWrapper(Err {
            error: Cow::Borrowed("Failed to fetch permissions from this club from the Database"),
            error_description: None
        })),
    };

    let perms = build_permission_from_res!(res);

    Response::Ok(AskamaWrapper(ClubDiscordPermissions{
        information: ClubInstance {
            name: res.name,
            code: res.code,
            path_name: res.path_name,
            permissions: perms,
            limits: limits.clone()
        },
        permissions: discord_perms.into_iter().map(|v|DiscordPermission{
            discord_id: v.discord_id.cast_unsigned(),
            discord_name: v.username,
            discord_discriminator: v.discriminator,
            permission: build_permission_from_res!(v),
        }).collect(),
    }))
}