use std::borrow::Cow;
use base64::Engine;
use tokio::sync::Mutex;
use crate::git::push::add_files_top;
use crate::modals::err::Err;
use crate::rocket::api::club::{Permissions, CLUB_OWNERS};
use crate::rocket::{AskamaWrapper, State};
use crate::rocket::auth::discord::JWT;
use crate::rocket::Response;

#[derive(Debug, serde_derive::Serialize)]
pub struct ClubInfo<'a>{
    name: &'a str,
    clubnames_allowed_to_replace: &'a [String],
    permissions: Vec<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    group_id: Option<&'a str>
}
#[derive(Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub struct ClubOwner{
    club_owners: Vec<Vec<String>>,
}

#[actix_web::post("/api/club/{club}/publish")]
pub async fn post_publish<'r>(
    auth: State<JWT>,
    repo: ::actix_web::web::Data<Mutex<git2::Repository>>,
    mk: State<crate::Keypair>,
    path: actix_web::web::Path<String>,
) -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    let club = path.into_inner();
    match Permissions::require_permission(&auth, &club, |v|v.submit).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }

    let db = crate::get_db().await;

    let permissions = match sqlx::query!(r#"
SELECT
    public.club_vrc_permission.permission_level,
    ARRAY_AGG(public.vrc_name.name) as permissions
FROM public.club_vrc_permission
INNER JOIN public.vrc_name on public.vrc_name.id = public.club_vrc_permission.vrc_name
INNER JOIN public.club on public.club_vrc_permission.club_id = public.club.id
WHERE public.club."path-name" = $1
GROUP BY public.club_vrc_permission.permission_level
"#, club)
        .fetch_all(&db)
        .await
    {
        Ok(v) => {
            let mut permissions = Vec::<Vec<String>>::new();
            for item in v {
                let level = item.permission_level as usize;
                if level >= permissions.len(){
                    permissions.resize(level + 1, Vec::new());
                }
                permissions[level] = item.permissions.unwrap_or_default();
            }
            permissions
        },
        Err(_) => return Response::Error(None, AskamaWrapper(Err{
            error: Cow::Borrowed("Failed to get vrchat permission data of the Club"),
            error_description: None,
        })),
    };


    let (data, res) = if club == CLUB_OWNERS {
        let data = ClubOwner{
            club_owners: permissions,
        };
        (serde_json::to_string(&data), None)
    } else {
        let res = match sqlx::query!(r#"
    WITH club_allowed_replace_name AS (
        SELECT
            public.club_allowed_replace.club_id,
            ARRAY_AGG(public.club."path-name") as allowed_replace
        FROM public.club_allowed_replace
        INNER JOIN public.club ON public.club.id = public.club_allowed_replace.switch_to_clubid
        GROUP BY public.club_allowed_replace.club_id
    )
    SELECT
        public.club."path-name" as path_name,
        public.club.id,
        public.club.group_id,
        COALESCE(club_allowed_replace_name.allowed_replace, ARRAY[]::text[]) as allowed_replace,
        public.club_logo.image as "club_logo?",
        public.club_poster1.image as "club_poster1?",
        public.club_poster2.image as "club_poster2?",
        public.club_poster3.image as "club_poster3?"
    FROM public.club
    INNER JOIN public.discord_permissions ON (public.club.id = public.discord_permissions.club_id OR public.discord_permissions.club_id = 0)
    LEFT JOIN club_allowed_replace_name ON club_allowed_replace_name.club_id = public.club.id
    LEFT JOIN public.club_logo ON public.club_logo.club_id = public.club.id
    LEFT JOIN public.club_poster1 ON public.club_poster1.club_id = public.club.id
    LEFT JOIN public.club_poster2 ON public.club_poster2.club_id = public.club.id
    LEFT JOIN public.club_poster3 ON public.club_poster3.club_id = public.club.id
    WHERE
        public.club."path-name" = $1 AND
        public.discord_permissions.discord_id = $2 AND
        public.discord_permissions.submit
    "#, &club, auth.get_user_id().cast_signed())
            .fetch_optional(&db)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return Response::Error(Some(actix_web::http::StatusCode::FORBIDDEN), AskamaWrapper(Err{
                error: Cow::Borrowed("Not Authorized or Club does not exist"),
                error_description: None,
            })),
            Err(_) => return Response::Error(Some(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR), AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to get data of the Club"),
                error_description: None,
            })),
        };
        let data = ClubInfo{
            name: &res.path_name,
            group_id: res.group_id.as_ref().map(|v|v.as_str()),
            clubnames_allowed_to_replace: res.allowed_replace.as_ref().map_or(&[], |v|v.as_slice()),
            permissions,
        };
        (serde_json::to_string(&data), Some(res))
    };

    let target_branch_name = format!("autopr/club/{club}");
    let commit_message = format!("Club {club} Update");
    let data = match data {
        Ok(mut v) => {
            //Text editors by default save a trailing newline.
            //For ease of verification let's do that too.
            v.push('\n');
            v
        },
        Err(e) => {
            tracing::warn!("Failed to serialize Club Security-List: {club} {e}");
            return Response::Err(None, Cow::Borrowed("Could not serialize data"));
        }
    };

    let mut bytes = String::with_capacity(
        data.len() +
            sphincsplus::CRYPTO_PUBLICKEYBYTES as usize /3*4 + 4 +
            sphincsplus::CRYPTO_BYTES as usize /3*4 + 4 +
            sphincsplus::CRYPTO_BYTES as usize /3*4 + 4
    );

    {
        let db = crate::get_db().await;
        let res = match sqlx::query!(r#"SELECT club.private_key, club.public_key FROM club WHERE "path-name"=$1 "#, club)
            .fetch_optional(&db)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return Response::Err(Some(actix_web::http::StatusCode::NOT_FOUND), Cow::Borrowed("")),
            Err(err) => {
                tracing::info!("Error querying db for private and public-key: {err}");
                return Response::Err(None, Cow::Borrowed("Error querying database"));
            },
        };

        let pk = res.public_key.as_slice().as_chunks();
        let sk = res.private_key.as_slice().as_chunks();
        if pk.1.len() > 0 {
            return Response::Err(None, Cow::Borrowed("Invalid Saved Public-Key"));
        }
        if sk.1.len() > 0 {
            return Response::Err(None, Cow::Borrowed("Invalid Saved Secret-Key"));
        }
        let pk = pk.0;
        let sk = sk.0;
        if pk.len() != 1 {
            return Response::Err(None, Cow::Borrowed("Invalid Saved Public-Key"));
        }
        if sk.len() != 1 {
            return Response::Err(None, Cow::Borrowed("Invalid Saved Secret-Key"));
        }
        let pk = pk[0];
        let sk = sk[0];

        let kp = crate::Keypair {
            public: pk,
            secret: sk,
        };
        bytes.push_str(&base64::engine::general_purpose::STANDARD.encode(&kp.public));
        bytes.push('\n');
        let kp_sig = match sphincsplus::crypto_sign_signature(&kp.public, &mk.secret) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to sign key: {club} {data:?} {e}");
                return Response::Err(None, Cow::Borrowed("Failed to sign key"));
            }
        };
        let ret = sphincsplus::crypto_sign_verify(&kp_sig, &kp.public, &mk.public);
        if ret != 0 {
            return Response::Err(None, Cow::Borrowed("Failed to verify signed key?"));
        }
        bytes.push_str(&base64::engine::general_purpose::STANDARD.encode(&kp_sig));
        bytes.push('\n');
        let data_sig = match sphincsplus::crypto_sign_signature(data.as_bytes(), &kp.secret) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to sign data: {club} {data:?} {e}");
                return Response::Err(None, Cow::Borrowed("Failed to sign data"));
            }
        };
        bytes.push_str(&base64::engine::general_purpose::STANDARD.encode(&data_sig));
        bytes.push('\n');
        bytes.push_str(&data);
    }
    let bytes = bytes;

    let repo = (&*repo).clone().lock_owned().await;
    let redir = format!("/auth/clubs/{club}");
    match tokio::task::spawn_blocking(move || {
        if let Some(res) = res {
            crate::git::push::push_files(
                &*repo,
                &target_branch_name,
                &club,
                vec![
                    (bytes.as_bytes(), "List.txt"),
                    (res.club_logo.as_ref().map_or(super::image::PLACEHOLDER_PNG, |v| v.as_slice()), "Logo.png"),
                    (res.club_poster1.as_ref().map_or(super::image::PLACEHOLDER_PNG, |v| v.as_slice()), "Poster1.png"),
                    (res.club_poster2.as_ref().map_or(super::image::PLACEHOLDER_PNG, |v| v.as_slice()), "Poster2.png"),
                    (res.club_poster3.as_ref().map_or(super::image::PLACEHOLDER_PNG, |v| v.as_slice()), "Poster3.png"),
                ],
                &commit_message,
            )
        } else {
            crate::git::push::add_commit(
                &*repo,
                &target_branch_name,
                |repo, builder| {
                    add_files_top(
                        vec![
                            (bytes.as_bytes(), "Owners.txt"),
                        ],
                        repo,
                        builder
                    )?;
                    Ok(())
                },
                &commit_message
            )
        }
    }).await {
        Ok(Ok(())) => {},
        Ok(Err(err)) => return Response::Err(None, err),
        Err(err) => return Response::Err(None, Cow::Owned(format!("Error Updating Repo: {err}"))),
    }

    Response::Redirect(None, redir.into())
}