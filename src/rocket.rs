use std::borrow::Cow;
use actix_web::dev::Payload;
use actix_web::HttpRequest;
use base64::Engine;
use crate::Keypair;

pub mod api;
pub mod auth;
pub mod club;

#[actix_web::get("/")]
pub async fn get_index() -> Response<actix_web::HttpResponse<core::convert::Infallible>> {
    Response::Redirect(None, "/clubs".into())
}
#[actix_web::get("/favicon.ico")]
pub async fn get_favicon() -> actix_web::HttpResponse<&'static [u8]> {
    let mut res = actix_web::HttpResponse::with_body(actix_web::http::StatusCode::OK, include_bytes!("../favicon.ico").as_slice());
    res.headers_mut().insert(actix_web::http::header::CONTENT_TYPE, actix_web::http::header::HeaderValue::from_static("image/x-icon"));
    res
}

pub struct AskamaWrapper<T>(pub T);
impl<T: askama::Template> AskamaWrapper<T> {
    pub fn render(self) -> Result<String, String> {
        self.0.render().map_err(|err| {
            format!(r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <meta name="color-scheme" content="light dark">
        <title>OAuth Error</title>
    </head>
    <body>
        <h1>Error Getting OAuth Token</h1>
        <p>Error: <code>{err}</code></p>
    </body>
"#)
        })
    }
}
impl<T: askama::Template> ::actix_web::Responder for AskamaWrapper<T> {
    type Body = String;
    fn respond_to(self, req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let mut v = match self.render() {
            Ok(v) => {
                v.respond_to(req)
            },
            Err(err) => {
                actix_web::HttpResponse::with_body(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, err)
            }
        };
        v.headers_mut().insert(actix_web::http::header::CONTENT_TYPE, actix_web::http::header::HeaderValue::from_static("text/html"));
        v
    }
}

#[derive(Debug, Clone)]
pub struct ETag<'a> {
    pub status: actix_web::http::StatusCode,
    pub data: Option<Cow<'a, [u8]>>,
    pub sha3_512: Cow<'a, [u8]>,
    pub header: actix_web::http::header::HeaderMap
}
impl actix_web::Responder for ETag<'static> {
    type Body = actix_web::body::EitherBody<Cow<'static, [u8]>, ()>;

    fn respond_to(self, _: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let mut response = actix_web::HttpResponse::with_body(
            if self.data.is_none() {
                actix_web::http::StatusCode::NOT_MODIFIED
            } else {
                self.status
            },
            self.data.map_or(actix_web::body::EitherBody::right(()), actix_web::body::EitherBody::left)
        );
        let header_base64 = base64::engine::general_purpose::STANDARD.encode(&self.sha3_512);
        for (name, value) in self.header.into_iter() {
            response.headers_mut().append(name, value);
        }
        *response.status_mut() = self.status;
        let etag = format!("sha3_512-{header_base64}");
        use actix_web::http::header::TryIntoHeaderValue;
        match actix_web::http::header::ETag(actix_web::http::header::EntityTag::new(false, etag)).try_into_value() {
            Ok(v) => response.headers_mut().append(<actix_web::http::header::ETag as actix_web::http::header::Header>::name(), v),
            Err(err) => {
                tracing::warn!("Tried to set an etag, but it wasn't valid? Since when is `sha3_512-{{some_base_64}}` not a valid header value?:: {err}");
            }
        }
        response
    }
}

pub struct IfNoneMatch(pub Vec<Vec<u8>>);
impl<'r> actix_web::FromRequest for IfNoneMatch {
    type Error = core::convert::Infallible;
    type Future = core::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let mut vec = Vec::new();
        for header in req.headers().get_all("If-None-Match") {
            let header = match header.to_str(){
                Ok(h) => h,
                Err(_) => continue,
            };
            for header in header.split(","){
                let header = header.trim();
                if header.starts_with("W/"){
                    continue;
                }
                let header = if let Some(v) = header.strip_prefix(r#"""#) {
                    if let Some(v) = v.strip_suffix(r#"""#) {
                        v
                    } else {
                        continue;
                    }
                } else {
                    header
                };
                if let Some(header) = header.strip_prefix("sha3_512-"){
                    if let Ok(v) = base64::engine::general_purpose::STANDARD.decode(header) {
                        vec.push(v);
                    }
                }

            }
        }
        core::future::ready(Ok(Self(vec)))
    }

}
pub enum Response<T> {
    Ok(T),
    Redirect(Option<actix_web::http::StatusCode>, Cow<'static, str>),
    Err(Option<actix_web::http::StatusCode>, Cow<'static, str>),
    Error(Option<actix_web::http::StatusCode>, AskamaWrapper<crate::modals::err::Err<'static>>),
}

impl<T:actix_web::Responder> actix_web::Responder for Response<T> {
    type Body = actix_web::body::EitherBody<actix_web::body::EitherBody<(), String>, T::Body>;

    fn respond_to(self, req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        match self {
            Self::Ok(v) => {
                v.respond_to(req).map_into_right_body()
            }
            Self::Redirect(code, location) => {
                let mut resp = actix_web::HttpResponse::with_body(code.unwrap_or(actix_web::http::StatusCode::TEMPORARY_REDIRECT), ());
                match actix_web::http::header::HeaderValue::from_str(&location) {
                    Ok(v) => resp.headers_mut().append(actix_web::http::header::LOCATION, v),
                    Err(err) => {
                        return Self::Error(None, AskamaWrapper(crate::modals::err::Err{
                            error: Cow::Borrowed("Failed to convert Location header to a valid Header Value"),
                            error_description: Some(err.to_string().into())
                        })).respond_to(req);
                    }
                }
                resp.map_into_left_body().map_into_left_body()
            }
            Self::Err(code, err) => {
                (err, code.unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)).respond_to(req).map_into_right_body().map_into_left_body()
            }
            Self::Error(code, wrapper) => {
                (wrapper, code.unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)).respond_to(req).map_into_right_body().map_into_left_body()
            }
        }
    }
}

pub struct State<T: ?Sized>(pub T);
impl<T: ?Sized> core::ops::Deref for State<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, T: Clone + 'static> actix_web::FromRequest for State<T> {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, actix_web::Error>>;

    #[inline]
    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        if let Some(st) = req.app_data::<T>() {
            std::future::ready(Ok(Self(st.clone())))
        } else {
            tracing::debug!(
                "Failed to extract `{}` for `{}` handler. For the Data extractor to work \
                correctly pass the data to `App::app_data()`. \
                Ensure that types align in both the set and retrieve calls.",
                core::any::type_name::<Keypair>(),
                req.match_name().unwrap_or_else(|| req.path())
            );

            std::future::ready(Err(actix_web::error::ErrorInternalServerError(
                "Requested application data is not configured correctly. \
                View/enable debug logs for more details.",
            )))
        }
    }
}