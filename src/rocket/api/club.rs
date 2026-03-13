use std::borrow::Cow;
use crate::modals::err::Err;
use crate::rocket::AskamaWrapper;
use crate::rocket::auth::discord::JWT;

pub mod image;
pub mod publish;
pub mod code_replacements;
pub mod vrcuser_level;
pub mod club_name;
pub mod manage_permissions;
pub mod new;

pub(crate) const CLUB_OWNERS:&str = "!CLUB-OWNERS";

#[serde_with::serde_as]
#[derive(Debug, serde_derive::Deserialize, serde_derive::Serialize)]
#[serde(default)]
pub struct Permissions{
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub add_discord_user: bool,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub remove_discord_user: bool,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub update_club_name: bool,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub add_allowed_code_replacements: bool,
    #[serde_as(as = "::serde_with::NoneAsEmptyString")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_level: Option<i16>,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub update_logo: bool,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub update_poster1: bool,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub update_poster2: bool,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub update_poster3: bool,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub remove_allowed_code_replacements: bool,
    #[serde_as(as = "::serde_with::NoneAsEmptyString")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_level: Option<i16>,
    #[serde_as(as = "::serde_with::NoneAsEmptyString")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manage_permissions: Option<i32>,
    #[serde_as(as = "crate::serialization::bool::WebBool")]
    pub submit: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            add_discord_user: false,
            remove_discord_user: false,
            update_club_name: false,
            add_allowed_code_replacements: false,
            add_level: None,
            update_logo: false,
            update_poster1: false,
            update_poster2: false,
            update_poster3: false,
            remove_allowed_code_replacements: false,
            remove_level: None,
            manage_permissions: None,
            submit: false,
        }
    }
}
macro_rules! build_permission_from_res {
    ($name:ident) => {
        $crate::rocket::api::club::Permissions::new(
            $name.add_discord_user,
            $name.remove_discord_user,
            $name.update_club_name,
            $name.add_allowed_code_replacements,
            $name.add_level,
            $name.update_logo,
            $name.update_poster1,
            $name.update_poster2,
            $name.update_poster3,
            $name.remove_allowed_code_replacements,
            $name.remove_level,
            $name.manage_permissions,
            $name.submit,
        )
    };
}
pub(crate) use build_permission_from_res;
impl Permissions {
    pub const fn new(
        add_discord_user: bool,
        remove_discord_user: bool,
        update_club_name: bool,
        add_allowed_code_replacements: bool,
        add_level: Option<i16>,
        update_logo: bool,
        update_poster1: bool,
        update_poster2: bool,
        update_poster3: bool,
        remove_allowed_code_replacements: bool,
        remove_level: Option<i16>,
        manage_permissions: Option<i32>,
        submit: bool,
    ) -> Self {
        Self{
            add_discord_user,
            remove_discord_user,
            update_club_name,
            add_allowed_code_replacements,
            add_level,
            update_logo,
            update_poster1,
            update_poster2,
            update_poster3,
            remove_allowed_code_replacements,
            remove_level,
            manage_permissions,
            submit,
        }
    }
    pub async fn get_from_db(discord_id: u64, path: &str) -> ::anyhow::Result<Option<Self>> {
        let db = crate::get_db().await;
        //Select 0, if a permission exists there, or from the club-perm
        //TODO: Not really a nice way of preferring the discord-permission of club_id 0 (if it exists) over the actual discord_permission with the actual club_id
        let res = sqlx::query!(r#"
    SELECT public.discord_permissions.* FROM public.club
        INNER JOIN public.discord_permissions ON public.discord_permissions.club_id = public.club.id OR public.discord_permissions.club_id = 0
        WHERE public.discord_permissions.discord_id = $1 AND public.club."path-name" = $2
        ORDER BY public.discord_permissions.club_id ASC
        LIMIT 1
        "#, discord_id.cast_signed(), path)
            .fetch_optional(&db)
            .await?;
        let res = match res{
            Some(res) => res,
            None => return Ok(None),
        };

        Ok(Some(Self{
            add_discord_user: res.add_discord_user,
            remove_discord_user: res.remove_discord_user,
            update_club_name: res.update_club_name,
            add_allowed_code_replacements: res.add_allowed_code_replacements,
            add_level: res.add_level,
            update_logo: res.update_logo,
            update_poster1: res.update_poster1,
            update_poster2: res.update_poster2,
            update_poster3: res.update_poster3,
            remove_allowed_code_replacements: res.remove_allowed_code_replacements,
            remove_level: res.remove_level,
            manage_permissions: res.manage_permissions,
            submit: res.submit,
        }))
    }
    #[inline]
    pub async fn get_from_jwt(jwt: &JWT, path: &str) -> ::anyhow::Result<Option<Self>> {
        Self::get_from_db(jwt.get_user_id(), path).await
    }
    #[inline]
    pub async fn get_from_jwt_err(jwt: &JWT, path: &str) -> Result<Option<Self>, Err<'static>> {
        match Self::get_from_jwt(jwt, path).await {
            Ok(v) => Ok(v),
            Err(err) => Err(Err{
                error: Cow::Borrowed("Failed to get permissions from db"),
                error_description: Some(Cow::Owned(err.to_string())),
            })
        }
    }
    #[inline]
    pub async fn require_permission(jwt: &JWT, path: &str, permission: impl FnOnce(&Self) -> bool) -> Result<(), (actix_web::http::StatusCode, AskamaWrapper<Err<'static>>)> {
        let err = (actix_web::http::StatusCode::UNAUTHORIZED, AskamaWrapper(Err{
            error: Cow::Borrowed("No Permission"),
            error_description: None,
        }));
        match Self::get_from_jwt_err(jwt, path).await {
            Ok(None) => Err(err),
            Ok(Some(v)) => if permission(&v) {
                Ok(())
            } else {
                Err(err)
            },
            Err(err) => Err((actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, AskamaWrapper(Err{
                error: Cow::Borrowed("Failed to get permissions from db"),
                error_description: Some(Cow::Owned(err.to_string())),
            }))),
        }
    }
}