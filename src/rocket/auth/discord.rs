use actix_web::HttpMessage;
pub mod new;
pub mod oauth;
pub mod err;
pub mod logout;

use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::num::NonZeroU16;
use std::str::FromStr;
use std::time::{UNIX_EPOCH};
use actix_web::body::BoxBody;
use crate::rocket::AskamaWrapper;

const DISCORD_TOKEN_COOKIE_NAME: &str = "discord_jwt";
#[derive(Clone)]
pub struct Discord {
    id: serenity::model::prelude::ApplicationId,
    secret: String,
    client: reqwest::Client,
    oauth_redirect_url: actix_web::http::uri::Uri,
}

pub async fn setup() -> ::anyhow::Result<Discord> {
    let discord_app_id = std::env::var("DISCORD_ID").map_err(|err|::anyhow::format_err!("Could not find DISCORD_ID: {err}"))?;
    let secret = std::env::var("DISCORD_SECRET").map_err(|err|::anyhow::format_err!("Could not find DISCORD_SECRET: {err}"))?;
    let return_url = std::env::var("DISCORD_OAUTH_RETURN_URL").map_err(|err|::anyhow::format_err!("Could not find DISCORD_OAUTH_RETURN_URL: {err}"))?;
    let return_url = actix_web::http::uri::Uri::try_from(return_url).map_err(|err|::anyhow::format_err!("Failed to parse DISCORD_OAUTH_RETURN_URL as an Absolute url: {err}"))?;
    let discord_app_id = match serenity::model::prelude::ApplicationId::from_str(discord_app_id.as_str()) {
        Ok(discord_app_id) => discord_app_id,
        Err(err) => anyhow::bail!("Failed to parse discord application ID: {err}")
    };

    Ok(Discord{
        id: discord_app_id,
        secret,
        client: reqwest::Client::new(),
        oauth_redirect_url: return_url,
    })
}

#[derive(Debug, Clone)]
pub struct JWT{
    jwt: JWTInternal,
}
impl core::ops::Deref for JWT {
    type Target = JWTInternal;
    fn deref(&self) -> &Self::Target {
        &self.jwt
    }
}
#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
#[non_exhaustive]
pub enum JWTInternal {
    V1(TokenMeta)
}

impl JWTInternal {
    pub fn is_valid(&self) -> ::anyhow::Result<bool> {
        match self {
            Self::V1(v) => v.is_valid(),
        }
    }

    #[inline]
    fn get_token(&self) -> &Token {
        match self {
            Self::V1(v) => v.get_token(),
        }
    }
    pub fn get_user_id(&self) -> u64 {
        match self {
            Self::V1(v) => v.get_user_id(),
        }
    }
    pub fn get_username(&self) -> &str {
        match self {
            Self::V1(v) => v.get_username(),
        }
    }
    pub fn get_display_name(&self) -> &str {
        match self {
            Self::V1(v) => v.get_display_name(),
        }
    }
    pub fn get_discriminator(&self) -> Option<NonZeroU16> {
        match self {
            Self::V1(v) => v.get_discriminator(),
        }
    }

    pub async fn refresh(&mut self, discord: &Discord) -> ::anyhow::Result<()> {
        match self {
            Self::V1(v) => v.refresh(discord).await,
        }
    }

    pub async fn new(discord: &Discord, code: &str) -> ::anyhow::Result<Self> {
        let token = ExchangeToken{
            grant_type: Cow::Borrowed("authorization_code"),
            code: Cow::Borrowed(code),
            redirect_uri: Cow::Owned(discord.oauth_redirect_url.to_string()),
        };
        let request = discord.client.post("https://discord.com/api/v10/oauth2/token")
            .basic_auth(discord.id.get(), Some(&discord.secret))
            .form(&token)
            .send()
            .await
            .map_err(|err| ::anyhow::format_err!("Failed to post-request to exchange token: {err}"))?
            .bytes()
            .await
            .map_err(|err| ::anyhow::format_err!("Failed to receive reply to exchange token post-request: {err}"))?
            ;

        let token = serde_json::from_slice::<Token>(&request)
            .map_err(|err| ::anyhow::format_err!("Failed to parse reply to exchange token post-request as an access-token: {err}"))?;

        let time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| ::anyhow::format_err!("Failed to get time since UNIX-EPOCH: {err}"))?;

        let expires_at = u64::saturating_add(time.as_secs(), token.expires_in);
        let mut v1 = TokenMeta{
            token,
            created_at: time.as_secs(),
            expires_at,
            user_id: 0,
            username: "".to_string(),
            discriminator: None,
            display_name: None,
            user_avatar_image_hash: None,
        };
        v1.update_user_info().await.map_err(|err| ::anyhow::format_err!("Failed to update user info: {err}"))?;
        let slf = Self::V1(v1);
        Ok(slf)
    }
}

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct TokenMeta {
    token: Token,
    created_at: u64,
    expires_at: u64,
    user_id: u64,
    username: String,
    discriminator: Option<NonZeroU16>,
    display_name: Option<String>,
    user_avatar_image_hash: Option<serenity::model::prelude::ImageHash>,
}
impl TokenMeta {
    pub fn is_valid(&self) -> ::anyhow::Result<bool> {
        let time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        Ok(time < self.expires_at)
    }
    #[inline]
    fn get_token(&self) -> &Token {
        &self.token
    }
    pub async fn refresh(&mut self, discord: &Discord) -> ::anyhow::Result<()> {
        let refresh_token = RefreshToken::from(self.get_token());
        let request = discord.client.post("https://discord.com/api/v10/oauth2/token")
            .basic_auth(discord.id.get(), Some(&discord.secret))
            .form(&refresh_token)
            .send()
            .await
            .map_err(|err| ::anyhow::format_err!("Failed to post-request to refresh token: {err}"))?
            .bytes()
            .await
            .map_err(|err| ::anyhow::format_err!("Failed to receive reply to refresh token post-request: {err}"))?
        ;

        let token = serde_json::from_slice(&request)
            .map_err(|err| ::anyhow::format_err!("Failed to parse reply to refresh token post-request as an access-token: {err}"))?;

        self.token = token;
        self.created_at = 0;
        self.expires_at = 0;
        let time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| ::anyhow::format_err!("Failed to get time since UNIX-EPOCH: {err}"))?;
        self.created_at = time.as_secs();
        self.expires_at = u64::saturating_add(time.as_secs(), self.token.expires_in);

        self.update_user_info().await.map_err(|err| ::anyhow::format_err!("Failed to update user info: {err}"))?;
        Ok(())
    }

    async fn update_user_info(&mut self) -> ::anyhow::Result<()> {
        let mut user_info = self.get_user_info().await?;
        self.user_id = user_info.id.get();
        self.username = core::mem::take(&mut user_info.name);
        self.discriminator = user_info.discriminator;
        self.display_name = core::mem::take(&mut user_info.global_name);
        self.user_avatar_image_hash = user_info.avatar;

        let db = crate::get_db().await;
        sqlx::query!(
            r#"INSERT INTO public.discord_info (user_id, username, discriminator, display_name) VALUES ($1, $2, $3, $4) on conflict (user_id) DO UPDATE SET username = $2, discriminator = $3, display_name = $4"#,
            self.user_id.cast_signed(),
            self.username,
            self.discriminator.map(|v|v.get().cast_signed()),
            self.display_name
        ).execute(&db).await
            .map_err(|err| {
                let err = ::anyhow::format_err!("Failed to update user info in db: {err}");
                tracing::error!("{err}");
                err
            })?;

        if let Some(avatar) = user_info.avatar {
            let hash = avatar.to_string();
            let hash = if avatar.is_animated() {
                match hash.strip_prefix("a_") {
                    Some(v) => v,
                    None => anyhow::bail!("Avatar hash was advertised as animated, but the serialized representation isn't prefixed with 'a_'.")
                }
            } else {
                hash.as_str()
            };
            sqlx::query!(
                r#"INSERT INTO discord_avatar_info (user_id, animated, image_hash) VALUES ($1, $2, decode($3, 'hex')) ON CONFLICT (user_id) DO UPDATE SET animated = $2, image_hash = decode($3, 'hex')"#,
                self.user_id.cast_signed(),
                avatar.is_animated(),
                hash,
            ).execute(&db).await
                .map_err(|err|  {
                    let err = ::anyhow::format_err!("Failed to update discord_avatar_info in db: {err}");
                    tracing::error!("{err}");
                    err
                })?;
        }
        Ok(())
    }
    async fn get_user_info(&self) -> ::anyhow::Result<serenity::model::prelude::CurrentUser> {
        let token = format!("{} {}", self.token.token_type, self.token.access_token);
        let http = serenity::http::Http::new(&token);
        http.get_current_user().await.map_err(|err| ::anyhow::format_err!("Failed to get the Current User: {err}"))
    }
    #[inline]
    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
    #[inline]
    pub fn get_username(&self) -> &str {
        self.username.as_str()
    }
    #[inline]
    pub fn get_display_name(&self) -> &str {
        self.display_name.as_ref().map_or(self.username.as_str(), |v|v.as_str())
    }
    #[inline]
    pub fn get_discriminator(&self) -> Option<NonZeroU16> {
        self.discriminator
    }
    pub fn get_avatar_url(&self) -> String {
        match &self.user_avatar_image_hash {
            Some(hash) => format!("https://cdn.discordapp.com/avatars/{}/{}.{}?size=1024", self.user_id, hash, if hash.is_animated() { "gif" } else { "webp" }),
            None => {
                let avatar_id = if let Some(discriminator) = self.discriminator {
                    discriminator.get() % 5 // Legacy username system
                } else {
                    ((self.user_id >> 22) % 6) as u16 // New username system
                };

                format!("https://cdn.discordapp.com/embed/avatars/{}.png", avatar_id)
            }
        }
    }
}

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub scope: String,
}
#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
struct RefreshToken<'a> {
    grant_type: Cow<'a, str>,
    refresh_token: Cow<'a, str>,
}
impl<'a> From<&'a Token> for RefreshToken<'a> {
    fn from(token: &'a Token) -> Self {
        Self{
            grant_type: Cow::Borrowed("refresh_token"),
            refresh_token: Cow::Borrowed(&token.refresh_token),
        }
    }
}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
struct ExchangeToken<'a> {
    grant_type: Cow<'a, str>,
    code: Cow<'a, str>,
    redirect_uri: Cow<'a, str>,
}

#[derive(Debug)]
pub enum AuthErr {
    NoCookie,
    DeserialisationError(::anyhow::Error),
    ValidationError(::anyhow::Error),
    NoDiscord,
    NoSecretKey,
    NoCookieSet,
    CookieHeaderInvalidValue(::actix_web::http::header::InvalidHeaderValue),
    RefreshError(::anyhow::Error),
    SerialisationError(::anyhow::Error),
}

impl Display for AuthErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthErr::NoCookie => write!(f, "No cookie cookie set"),
            AuthErr::DeserialisationError(err) => write!(f, "Failed to deserialize cookie value: {}", err),
            AuthErr::ValidationError(err) => write!(f, "Failed to validate if the cookie has expired: {}", err),
            AuthErr::NoDiscord => write!(f, "No Discord OAuth information found"),
            AuthErr::NoSecretKey => write!(f, "No Secret Key for Private Cookies found"),
            AuthErr::NoCookieSet => write!(f, "Could not get Cookie, even though we just created one. WHO IS EATING MY COOKIES?!?"),
            AuthErr::CookieHeaderInvalidValue(err) => write!(f, "There was an error converting the Cookie to a Set-Header value: {err}"),
            AuthErr::RefreshError(err) => write!(f, "Error whilst refreshing Discord-OAuth access-token: {}", err),
            AuthErr::SerialisationError(err) => write!(f, "Error whilst Serializing Discord-OAuth information: {}", err),
        }
    }
}

impl std::error::Error for AuthErr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self {
            AuthErr::DeserialisationError(err) => Some(&**err),
            AuthErr::SerialisationError(err) => Some(&**err),
            AuthErr::ValidationError(err) => Some(&**err),
            AuthErr::RefreshError(err) => Some(&**err),
            AuthErr::CookieHeaderInvalidValue(err) => Some(err),
            _ => None,
        }
    }
}
impl actix_web::ResponseError for AuthErr {
    fn error_response(&self) -> actix_web::HttpResponse<BoxBody> {
        let content = AskamaWrapper(match self {
            Self::NoCookie => NeedLogin {
                description: Cow::Borrowed("It seems like the cookie with Discord-Auth information doesn't exist? Did the cookie monster eat it?!?"),
            },
            Self::DeserialisationError(_) => NeedLogin {
                description: Cow::Borrowed("The Discord-Auth information seems malformed.")
            },
            Self::ValidationError(_) => NeedLogin {
                description: Cow::Borrowed("Failed to check the Discord-Auth information's Validity. Is the Clock of the Server on-time?")
            },
            Self::NoDiscord => NeedLogin {
                description: Cow::Borrowed("Failed to get some information regarding Discord-OAuth.")
            },
            Self::NoSecretKey => NeedLogin {
                description: Cow::Borrowed("Failed to get the Secret-Key information for Private Cookies.")
            },
            Self::NoCookieSet => NeedLogin {
                description: Cow:: Borrowed("Could not get Cookie, even though we just created one. WHO IS EATING MY COOKIES?!?")
            },
            Self::CookieHeaderInvalidValue(_) => NeedLogin {
                description: Cow:: Borrowed("There was an error converting the Cookie to a Set-Header value. Likely the resulting HTTP-Header was invalid."),
            },
            Self::RefreshError(_) => NeedLogin {
                description: Cow::Borrowed("Failed to refresh the expired Discord-Auth information.")
            },
            Self::SerialisationError(_) => NeedLogin {
                description: Cow::Borrowed("Failed to serialize the new Discord-Auth information.")
            },
        }).render();
        actix_web::HttpResponse::InternalServerError()
            .insert_header((actix_web::http::header::CONTENT_TYPE, actix_web::http::header::HeaderValue::from_static("text/html")))
            .body(content.map_or_else(core::convert::identity, core::convert::identity))
    }
}

#[derive(askama::Template)]
#[template(path = "api/auth/discord/need-login.html")]
struct NeedLogin{
    description: Cow<'static, str>,
}

impl actix_web::Responder for AuthErr {
    type Body = String;

    fn respond_to(self, req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        (AskamaWrapper(match self {
            Self::NoCookie => NeedLogin {
                description: Cow::Borrowed("It seems like the cookie with Discord-Auth information doesn't exist? Did the cookie monster eat it?!?"),
            },
            Self::DeserialisationError(_) => NeedLogin {
                description: Cow::Borrowed("The Discord-Auth information seems malformed.")
            },
            Self::ValidationError(_) => NeedLogin {
                description: Cow::Borrowed("Failed to check the Discord-Auth information's Validity. Is the Clock of the Server on-time?")
            },
            Self::NoDiscord => NeedLogin {
                description: Cow::Borrowed("Failed to get some information regarding Discord-OAuth.")
            },
            Self::NoSecretKey => NeedLogin {
                description: Cow::Borrowed("Failed to get the Secret-Key information for Private Cookies.")
            },
            Self::NoCookieSet => NeedLogin {
                description: Cow:: Borrowed("Could not get Cookie, even though we just created one. WHO IS EATING MY COOKIES?!?")
            },
            Self::CookieHeaderInvalidValue(_) => NeedLogin {
                description: Cow:: Borrowed("There was an error converting the Cookie to a Set-Header value. Likely the resulting HTTP-Header was invalid."),
            },
            Self::RefreshError(_) => NeedLogin {
                description: Cow::Borrowed("Failed to refresh the expired Discord-Auth information.")
            },
            Self::SerialisationError(_) => NeedLogin {
                description: Cow::Borrowed("Failed to serialize the new Discord-Auth information.")
            },
        }), actix_web::http::StatusCode::UNAUTHORIZED).respond_to(req)
    }
}

pub async fn my_middleware(
    req: actix_web::dev::ServiceRequest,
    next: actix_web::middleware::Next<impl actix_web::body::MessageBody + 'static>,
) -> Result<actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>, actix_web::Error> {
    let cookie = match req.cookie(DISCORD_TOKEN_COOKIE_NAME) {
        Some(v) => v,
        None => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::NoCookie, req.request().clone())),
    };
    let key = match req.app_data(){
        Some(v) => v,
        None => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::NoSecretKey, req.request().clone())),
    };
    let cookie = {
        let mut jar = actix_web::cookie::CookieJar::new();
        jar.add_original(cookie);
        let private = jar.private(key);

        match private.get(DISCORD_TOKEN_COOKIE_NAME) {
            Some(cookie) => cookie,
            None => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::NoCookieSet, req.request().clone())),
        }
    };
    let cookie = cookie.value();
    let cookie = cookie.strip_prefix(r#"""#).unwrap_or(cookie);
    let cookie = cookie.strip_suffix(r#"""#).unwrap_or(cookie);

    let mut jwt = match serde_json::from_str::<JWTInternal>(cookie) {
        Ok(v) => v,
        Err(err) => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::DeserialisationError(err.into()), req.request().clone())),
    };
    let cookie = if !match jwt.is_valid() {
        Ok(v) => v,
        Err(err) => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::ValidationError(err), req.request().clone())),
    } {
        let discord = match req.app_data::<Discord>() {
            Some(v) => v,
            None => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::NoDiscord, req.request().clone())),
        };
        match jwt.refresh(discord).await {
            Ok(()) => {},
            Err(err) => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::RefreshError(err), req.request().clone())),
        }
        let jwt_str = match serde_json::to_string(&jwt) {
            Ok(v) => v,
            Err(err) => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::SerialisationError(err.into()), req.request().clone())),
        };

        let mut jar = actix_web::cookie::CookieJar::new();
        {
            let mut jar = jar.private_mut(key);
            let mut cookie = actix_web::cookie::Cookie::new(DISCORD_TOKEN_COOKIE_NAME, jwt_str);
            cookie.set_secure(true);
            jar.add(cookie)
        }
        let cookie = match jar.get(DISCORD_TOKEN_COOKIE_NAME) {
            Some(cookie) => cookie,
            None => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::NoCookieSet, req.request().clone())),
        };
        let cookie = match actix_web::http::header::HeaderValue::from_str(&cookie.to_string()) {
            Ok(v) => v,
            Err(err) => return Ok(actix_web::dev::ServiceResponse::from_err(AuthErr::CookieHeaderInvalidValue(err), req.request().clone())),
        };

        req.extensions_mut().insert(JWT{
            jwt,
        });
        Some(cookie)
    } else {
        req.extensions_mut().insert(JWT{
            jwt,
        });
        None
    };

    let mut resp = next.call(req).await?;

    // post-processing
    if let Some(cookie) = cookie {
        resp.response_mut().headers_mut().append(actix_web::http::header::SET_COOKIE, cookie);
    }

    Ok(resp.map_body(|_, body| body.boxed()))
}