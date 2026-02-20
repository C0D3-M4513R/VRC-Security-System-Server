use std::borrow::Cow;
use std::cmp::min;
use std::sync::LazyLock;
use crate::rocket::{AskamaWrapper, ETag, IfNoneMatch, Response, State};
use crate::rocket::auth::discord::JWT;
use crate::modals::err::Err;
use crate::rocket::api::club::Permissions;
#[derive(serde_derive::Deserialize)]
pub struct Upload {
    file: Vec<u8>,
}
pub struct PutImageResponse{
    location: String
}

impl actix_web::Responder for PutImageResponse {
    type Body = actix_web::body::EitherBody<(), <Response<actix_web::HttpResponse<core::convert::Infallible>> as actix_web::Responder>::Body>;

    fn respond_to(self, req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let mut resp = actix_web::HttpResponse::with_body(actix_web::http::StatusCode::TEMPORARY_REDIRECT, actix_web::body::EitherBody::left(()));
        resp.headers_mut().insert(actix_web::http::header::CACHE_CONTROL, actix_web::http::header::HeaderValue::from_static("no-cache"));
        match actix_web::http::header::HeaderValue::from_str(&self.location) {
            Ok(v) => resp.headers_mut().append(actix_web::http::header::LOCATION, v),
            Err(err) => {
                return Response::<actix_web::HttpResponse<core::convert::Infallible>>::Error(None, AskamaWrapper(crate::modals::err::Err{
                    error: Cow::Borrowed("Failed to convert Location header to a valid Header Value"),
                    error_description: Some(err.to_string().into())
                })).respond_to(req).map_into_right_body()
            }
        }
        resp
    }
}

#[actix_web::post("/api/club/<club>/image/<name>")]
pub async fn put_image<'r>(auth: State<JWT>, club: String, name: String, data: actix_web::web::Form<Upload>) -> Response<PutImageResponse> {
    let permission:fn(&Permissions)->bool = match name.as_str() {
        "Logo.png" => |v|v.update_logo,
        "Poster1.png" => |v|v.update_poster1,
        "Poster2.png" => |v|v.update_poster2,
        "Poster3.png" => |v|v.update_poster3,
        _ => return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
            error: Cow::Borrowed("Invalid image name"),
            error_description: None
        })),
    };
    match Permissions::require_permission(&auth, &club, permission).await {
        Ok(()) => {}
        Err((code, err)) => return Response::Error(Some(code), err),
    }

    //The VRChat Limit is 32 MB (do they mean MB or MiB?).
    //We choose a limit less than that, just to be safe.
    if data.file.len() > 30*1000*1000 {
        return Response::Error(Some(actix_web::http::StatusCode::PAYLOAD_TOO_LARGE), AskamaWrapper(Err{
            error: Cow::Borrowed("Request data was too large? Please upload smaller files."),
            error_description: None,
        }));
    }
    let bytes = {
        use image::GenericImageView;

        fn helper<T: ::std::io::BufRead + ::std::io::Seek>(mut image: image::ImageReader<T>) -> Result<Vec<u8>, (Option<actix_web::http::StatusCode>, AskamaWrapper<Err<'static>>)> {
            let limits = ::image::Limits::default();
            #[cfg(feature = "restrict_image_upload_dimensions")]
            let limits = {
                let mut limits = limits;
                limits.max_image_width = Some(2048);
                limits.max_image_height = Some(2048);
                limits
            };
            image.limits(limits);
            let image = match image.with_guessed_format() {
                Ok(v) => v,
                Err(err) => return Err((Some(actix_web::http::StatusCode::PAYLOAD_TOO_LARGE), AskamaWrapper(Err{
                    error: Cow::Borrowed("Io error, whilst guessing image format:"),
                    error_description: Some(Cow::Owned(err.to_string()))
                }))),
            };
            let image = match image.decode() {
                Ok(image) => image,
                Err(image::ImageError::Limits(limit)) => return Err((Some(actix_web::http::StatusCode::PAYLOAD_TOO_LARGE), AskamaWrapper(Err{
                    error: Cow::Borrowed("Hit limit whilst decoding an image:"),
                    error_description: Some(Cow::Owned(limit.to_string()))
                }))),
                Err(err) => return Err((Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
                    error: Cow::Borrowed("Image decode error"),
                    error_description: Some(Cow::Owned(err.to_string()))
                }))),
            };

            let image =
                if image.dimensions().0 > 2048 || image.dimensions().1 > 2048 {
                    image.resize(min(2048, image.dimensions().0), min(2048, image.dimensions().1), image::imageops::FilterType::CatmullRom)
                } else {
                    image
                };

            if image.dimensions().0 > 2048 {
                return Err((None, AskamaWrapper(Err{
                    error: Cow::Borrowed("Image was too wide even after resize (max dimensions 2048x2048)"),
                    error_description: Some(Cow::Owned(format!("Actual: {}", image.dimensions().0))),
                })));
            }
            if image.dimensions().1 > 2048 {
                return Err((None, AskamaWrapper(Err{
                    error: Cow::Borrowed("Image was too high even after resize (max dimensions 2048x2048)"),
                    error_description: Some(Cow::Owned(format!("Actual: {}", image.dimensions().1))),
                })));
            }

            let mut buf = Vec::new();
            match image.write_with_encoder(image::codecs::png::PngEncoder::new(&mut buf)) {
                Ok(()) => {}
                Err(err) => {
                    return Err((None, AskamaWrapper(Err{
                        error: Cow::Borrowed("Failed to encode image as png"),
                        error_description: Some(Cow::Owned(err.to_string())),
                    })))
                }
            }

            Ok(buf)
        }
        let data = data.into_inner();
        let image = match tokio::task::block_in_place(||helper(image::ImageReader::new(std::io::Cursor::new(data.file)))) {
            Ok(v) => v,
            Err((code, err)) => return Response::Error(code, err),
        };

        image
    };

    let redir_url = format!("/clubs/{club}");
    let redir = Response::Redirect(None, redir_url.clone().into());
    let db = crate::get_db().await;
    let table = match match name.as_str() {
        "Logo.png" => sqlx::query!(r#"SELECT change_logo($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        "Poster1.png" => sqlx::query!(r#"SELECT change_poster1($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        "Poster2.png" => sqlx::query!(r#"SELECT change_poster2($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        "Poster3.png" => sqlx::query!(r#"SELECT change_poster3($1, $2, $3) as "digest!""#, auth.get_user_id().cast_signed(), club, bytes.as_slice()).fetch_optional(&db).await.map(|v|v.map(|v|v.digest)),
        _ => return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err {
            error: Cow::Borrowed("Invalid image name"),
            error_description: None,
        })),
    }{
        Ok(Some(v)) => v,
        Ok(None) => return redir,
        Err(err) => {
            tracing::error!("Failed to update Image: {err}");
            return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
                error: Cow::Borrowed("Failed up update Image in DB"),
                error_description: Some(err.to_string().into()),
            }))
        }
    };

    Response::Ok(PutImageResponse{
        location: redir_url,
    })
}

pub const PLACEHOLDER_PNG:&[u8] = include_bytes!("../../../../1x1-00000000.png");
pub const PLACEHOLDER_PNG_SHA3_512:&[u8] = include_bytes!("../../../../1x1-00000000.png.sha3-512");

pub static PLACEHOLDER_PNG_ETAG:LazyLock<ETag<'static>> = LazyLock::new(||ETag {
    status: actix_web::http::StatusCode::OK,
    data: Some(Cow::Borrowed(PLACEHOLDER_PNG)),
    sha3_512: Cow::Borrowed(PLACEHOLDER_PNG_SHA3_512),
    header: actix_web::http::header::HeaderMap::new(),
});

#[actix_web::get("/api/club/<club>/image/<name>")]
pub async fn get_image<'r>(_auth: State<JWT>, etag: IfNoneMatch, club: String, name: String) -> Response<ETag<'static>> {
    let db = crate::get_db().await;
    macro_rules! make_etag {
        ($ident:ident) => {
            ETag{
                status: actix_web::http::StatusCode::OK,
                data: Some(Cow::Owned($ident.image)),
                sha3_512: Cow::Owned($ident.digest),
                header: Default::default(),
            }
        };
        (etag, $ident:ident) => {
            ETag{
                status: actix_web::http::StatusCode::OK,
                data: None,
                sha3_512: Cow::Owned($ident.digest),
                header: Default::default(),
            }
        };
    }
    for i in etag.0 {
        let resp = |i|{
            let mut header = actix_web::http::header::HeaderMap::new();
            header.insert(actix_web::http::header::CACHE_CONTROL, actix_web::http::header::HeaderValue::from_static("no-cache"));
            header.insert(actix_web::http::header::CONTENT_TYPE, actix_web::http::header::HeaderValue::from_static("image/png"));
            Response::Ok(ETag{
                status: actix_web::http::StatusCode::OK,
                data: None,
                sha3_512: Cow::Owned(i),
                header,
            })
        };
        match match name.as_str() {
            "Logo.png" => sqlx::query!(r#"SELECT true as dummy FROM club_logo INNER JOIN club ON club_logo.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            "Poster1.png" => sqlx::query!(r#"SELECT true as dummy FROM club_poster1 INNER JOIN club ON club_poster1.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            "Poster2.png" => sqlx::query!(r#"SELECT true as dummy FROM club_poster2 INNER JOIN club ON club_poster2.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            "Poster3.png" => sqlx::query!(r#"SELECT true as dummy FROM club_poster3 INNER JOIN club ON club_poster3.club_id = club.id WHERE club."path-name" = $1 AND digest = $2 "#, club, i.as_slice()).fetch_optional(&db).await.map(|v|v.map(|_|())),
            _ => return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
                error: Cow::Borrowed("Invalid image name"),
                error_description: None,
            })),
        }{
            Ok(Some(())) => return resp(i),
            Ok(None) => {},
            Err(err) => {
                tracing::error!("Failed to get Image: {err}");
                return Response::Error(None, AskamaWrapper(Err{
                    error: Cow::Borrowed("Failed get Image from DB"),
                    error_description: Some(err.to_string().into()),
                }));
            }
        }
    }
    let mut etag:ETag = match match name.as_str() {
        "Logo.png" => sqlx::query!(r#"SELECT image, digest FROM club_logo INNER JOIN club ON club_logo.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        "Poster1.png" => sqlx::query!(r#"SELECT image, digest FROM club_poster1 INNER JOIN club ON club_poster1.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        "Poster2.png" => sqlx::query!(r#"SELECT image, digest FROM club_poster2 INNER JOIN club ON club_poster2.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        "Poster3.png" => sqlx::query!(r#"SELECT image, digest FROM club_poster3 INNER JOIN club ON club_poster3.club_id = club.id WHERE club."path-name" = $1"#, club).fetch_optional(&db).await.map(|v|v.map(|v|make_etag!(v))),
        _ => return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
            error: Cow::Borrowed("Invalid image name"),
            error_description: None,
        })),
    }{
        Ok(Some(v)) => v,
        Ok(None) => PLACEHOLDER_PNG_ETAG.clone(),
        Err(err) => {
            tracing::error!("Failed to get Image: {err}");
            return Response::Error(Some(actix_web::http::StatusCode::BAD_REQUEST), AskamaWrapper(Err{
                error: Cow::Borrowed("Failed get Image from DB"),
                error_description: Some(err.to_string().into()),
            }));
        }
    };

    etag.header.insert(actix_web::http::header::CONTENT_TYPE, actix_web::http::header::HeaderValue::from_static("image/png"));

    etag.header.insert(actix_web::http::header::CACHE_CONTROL, actix_web::http::header::HeaderValue::from_static("no-cache"));
    Response::Ok(etag)
}