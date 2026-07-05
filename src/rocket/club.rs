pub mod instance;
pub mod vrchat_permissions;
pub mod manage_permissions;

use std::borrow::Cow;
use std::sync::Arc;
use crate::{Keypair, INITIALIZING};
use crate::rocket::{AskamaWrapper, Response, State};
use crate::rocket::auth::discord::JWT;
use crate::rocket::api::club::build_permission_from_res;
use crate::modals::err::Err;
use crate::modals::clubs::{Club, Clubs};
#[actix_web::get("/")]
pub async fn get_index() -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    Response::Redirect(None, "clubs".into())
}
#[actix_web::get("/clubs")]
pub async fn get_club<'r>(auth: State<JWT>, keypair: State<Keypair>) -> Response<AskamaWrapper<Clubs>> {
    let db = crate::get_db().await;
    if INITIALIZING.swap(false, core::sync::atomic::Ordering::AcqRel) {
        match sqlx::query!(
            r#"SELECT add_initial_club($1, $2, $3, $4)"#,
            auth.get_user_id().cast_signed(), &keypair.public, &keypair.secret, None::<&str>
        )
            .execute(&db)
            .await
        {
            Ok(_) => {},
            Err(err) => {
                tracing::warn!("Failed to initialize DB with first club: {err}");
                INITIALIZING.store(true, core::sync::atomic::Ordering::Release);
                return Response::Error(None, AskamaWrapper(Err{
                    error: Cow::Borrowed("Failed to initialize DB with first club"),
                    error_description: None
                }));
            }
        }
    }
    let res = match sqlx::query!(r#"
        SELECT
            public.club."path-name" as path_name,
            public.club.code as code,
            public.club.name as name
        FROM club
            INNER JOIN public.discord_permissions on public.club.id = public.discord_permissions.club_id OR public.discord_permissions.club_id = 0
            WHERE public.discord_permissions.discord_id = $1
            ORDER BY id
    "#, auth.get_user_id().cast_signed()).fetch_all(&db)
        .await {
        Ok(res) => res,
        Err(_) => return Response::Error(None, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to fetch your permissions across club's from the Database"),
            error_description: None
        })),
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
        Err(_) => return Response::Error(None, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to fetch your permissions across from the Database"),
            error_description: None
        })),
    };

    Response::Ok(AskamaWrapper(Clubs::new(
        res.into_iter().map(|c| Club{path_name: Arc::from(c.path_name), code: c.code.cast_unsigned(), name: Arc::from(c.name)}).collect(),
        permission
    )))
}